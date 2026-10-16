use revolt_result::Result;
use ulid::Ulid;
use std::collections::{HashMap, HashSet};

use iso8601_timestamp::Timestamp;
use revolt_config::config;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};

use crate::util::permissions::DatabasePermissionQuery;
use crate::{Channel, Database, Member, Message, Server, User};

auto_derived!(
    /// Notification hub
    pub struct NotificationCenter {
        /// Why this notification exists
        pub kind: InboxKind,
        /// The mentioned user ID
        pub user_id: String,
        /// Message ID
        pub message_id: String,
        /// Channel ID
        pub channel_id: String,
        /// Server ID
        #[serde(skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
    }

    pub struct InboxOptions {
        pub limit: i64,
        pub before: Option<String>,
        pub mentions: bool,
        pub roles: bool,
        pub everyone: bool,
    }

    #[derive(Copy, PartialOrd, Ord)]
    pub enum InboxKind {
        Mention,
        Role,
        Everyone,
    }

    pub struct InboxItem {
        pub kind: InboxKind,
        pub message_id: String,
        pub channel_id: String,
        pub server_id: Option<String>,
        pub message: Option<Message>,
    }

    pub struct InboxPage {
        pub items: Vec<InboxItem>,
        /// Pass as `before` to continue; None when nothing older exists
        pub next_before: Option<String>,
    }
);

const MAX_ROUNDS: usize = 5;

fn ulid_at(ms: u64) -> String {
    Ulid::from_parts(ms, 0).to_string()
}

#[allow(clippy::disallowed_methods)]
impl NotificationCenter {
    /// Build a notification for one user
    fn new(
        kind: InboxKind,
        user_id: String,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
    ) -> NotificationCenter {
        NotificationCenter {
            kind,
            user_id,
            message_id: message_id.to_string(),
            channel_id: channel_id.to_string(),
            server_id: server_id.map(str::to_string),
        }
    }

    /// Create notifications for everyone in a message's (already permission-filtered) mentions
    pub async fn create_for_mentions(
        db: &Database,
        author_id: &str,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
        mentions: &[String],
    ) -> Result<()> {
        let notifications: Vec<NotificationCenter> = mentions
            .iter()
            .filter(|id| id.as_str() != author_id)
            .map(|id| NotificationCenter::new(InboxKind::Mention, id.clone(), message_id, channel_id, server_id))
            .collect();

        db.insert_notifications(notifications).await
    }

    /// Sync notifications after a message edit: drop removed mentions, add new ones
    pub async fn update_for_edit(
        db: &Database,
        author_id: &str,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
        old_mentions: &[String],
        new_mentions: &[String],
    ) -> Result<()> {
        let removed: Vec<String> = old_mentions
            .iter()
            .filter(|id| !new_mentions.contains(id))
            .cloned()
            .collect();
        let added: Vec<String> = new_mentions
            .iter()
            .filter(|id| !old_mentions.contains(id))
            .cloned()
            .collect();

        db.delete_notifications_for_users(message_id, &removed).await?;
        NotificationCenter::create_for_mentions(db, author_id, message_id, channel_id, server_id, &added)
            .await
    }

    /// Delete all notifications for a message
    pub async fn delete_for_message(db: &Database, message_id: &str) -> Result<()> {
        db.delete_notifications_for_message(message_id).await
    }

    /// Delete all notifications for multiple messages
    pub async fn delete_for_messages(db: &Database, message_ids: &[String]) -> Result<()> {
        db.delete_notifications_for_messages(message_ids).await
    }

    pub async fn delete_for_channels(db: &Database, channel_ids: &[String]) -> Result<()> {
        db.delete_notifications_for_channels(channel_ids).await
    }

    pub async fn delete_for_user(db: &Database, user_id: &str) -> Result<()> {
        db.delete_notifications_for_user(user_id).await
    }

    pub async fn delete_for_user_in_channel(
        db: &Database,
        user_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        db.delete_notifications_for_user_in_channel(user_id, channel_id).await
    }

    pub async fn delete_for_user_in_server(
        db: &Database,
        user_id: &str,
        server_id: &str
    ) -> Result<()> {
        db.delete_notifications_for_user_in_server(user_id, server_id).await
    }

    /// Delete notifications older than the retention window, returns how many were removed
    pub async fn prune(db: &Database) -> Result<u64> {
        let retention_days = config().await.features.limits.global.notification_retention_days;
        if retention_days == 0 {
            return Ok(0);
        }

        let cutoff =
            bson::DateTime::now().timestamp_millis() - retention_days as i64 * 86_400_000;
        db.delete_notifications_before(cutoff).await
    }

    pub async fn fetch_inbox(db: &Database, user: &User, opts: InboxOptions) -> Result<InboxPage> {
        let limit = opts.limit.clamp(1, 100);

        // Retention window: older rows are hidden whether or not the prune has run yet
        let days = config().await.features.limits.global.notification_retention_days;
        let cutoff = if days == 0 {
            ulid_at(0)
        } else {
            let ms = bson::DateTime::now().timestamp_millis() - days as i64 * 86_400_000;
            ulid_at(ms.max(0) as u64)
        };

        // Only loaded when role / everyone mentions are requested
        let mut members_by_server: HashMap<String, Member> = HashMap::new();
        let mut joined: HashMap<String, String> = HashMap::new();
        let mut role_ids: Vec<String> = vec![];
        let mut channel_to_server: HashMap<String, String> = HashMap::new();
        let mut channel_ids: Vec<String> = vec![];

        if opts.roles || opts.everyone {
            let members = db.fetch_all_memberships(&user.id).await?;

            joined = members
                .iter()
                .map(|m| {
                    let ms = m
                        .joined_at
                        .duration_since(Timestamp::UNIX_EPOCH)
                        .whole_milliseconds()
                        .max(0) as u64;
                    (m.id.server.clone(), ulid_at(ms))
                })
                .collect();

            role_ids = members.iter().flat_map(|m| m.roles.clone()).collect();

            if opts.everyone {
                let server_ids: Vec<String> = members.iter().map(|m| m.id.server.clone()).collect();
                for (server_id, channels) in db.fetch_servers_channel_ids(&server_ids).await? {
                    for channel in channels {
                        channel_to_server.insert(channel.clone(), server_id.clone());
                        channel_ids.push(channel);
                    }
                }
            }

            members_by_server = members
                .into_iter()
                .map(|m| (m.id.server.clone(), m))
                .collect();
        }

        // Filled lazily from the pages actually returned, cached across rounds
        let mut servers_by_id: HashMap<String, Server> = HashMap::new();
        let mut channels: HashMap<String, Channel> = HashMap::new();
        let mut can_view: HashMap<String, bool> = HashMap::new();

        let mut out: Vec<InboxItem> = vec![];
        let mut cursor = opts.before.clone();
        let mut exhausted = false;

        for _ in 0..MAX_ROUNDS {
            let before = cursor.as_deref();
            let mut raw: Vec<InboxItem> = vec![];

            // Raw candidates: no filtering yet, so the "exhausted" check below stays honest
            if opts.mentions {
                for n in db.fetch_notifications(&user.id, &cutoff, before, limit).await? {
                    raw.push(InboxItem {
                        kind: InboxKind::Mention,
                        message_id: n.message_id,
                        channel_id: n.channel_id,
                        server_id: n.server_id,
                        message: None,
                    });
                }
            }
            if opts.roles {
                for m in db
                    .fetch_role_mention_messages(&role_ids, &user.id, &cutoff, before, limit)
                    .await?
                {
                    raw.push(InboxItem::from_message(InboxKind::Role, m, &channel_to_server));
                }
            }
            if opts.everyone {
                let mut all: Vec<Message> = vec![];
                for chunk in channel_ids.chunks(500) {
                    all.extend(
                        db.fetch_everyone_mention_messages(chunk, &user.id, &cutoff, before, limit)
                            .await?,
                    );
                }
                all.sort_by(|a, b| b.id.cmp(&a.id));
                all.truncate(limit as usize);

                for m in all {
                    raw.push(InboxItem::from_message(InboxKind::Everyone, m, &channel_to_server));
                }
            }

            // Newest first, one entry per message (Mention beats Role beats Everyone)
            raw.sort_by(|a, b| b.message_id.cmp(&a.message_id).then(a.kind.cmp(&b.kind)));
            raw.dedup_by(|a, b| a.message_id == b.message_id);

            // Fewer than `limit` distinct candidates means every source ran dry
            exhausted = raw.len() < limit as usize;
            raw.truncate(limit as usize);

            let Some(oldest) = raw.last().map(|i| i.message_id.clone()) else {
                break;
            };
            cursor = Some(oldest); // next round continues below everything scanned so far

            // Load message bodies for direct rows
            let missing: Vec<String> = raw
                .iter()
                .filter(|i| i.message.is_none())
                .map(|i| i.message_id.clone())
                .collect();
            if !missing.is_empty() {
                let mut fetched: HashMap<String, Message> = db
                    .fetch_messages_by_id(&missing)
                    .await?
                    .into_iter()
                    .map(|m| (m.id.clone(), m))
                    .collect();
                for item in raw.iter_mut().filter(|i| i.message.is_none()) {
                    item.message = fetched.remove(&item.message_id);
                }
            }

            // Filter: deleted messages, and silent role / everyone pings
            raw.retain(|i| match &i.message {
                None => false,
                Some(message) => {
                    i.kind == InboxKind::Mention || !message.has_suppressed_notifications()
                }
            });

            // Fetch channels not seen yet
            let unseen: Vec<String> = raw
                .iter()
                .map(|i| i.channel_id.clone())
                .filter(|c| !channels.contains_key(c))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            if !unseen.is_empty() {
                for channel in db.fetch_channels(&unseen).await? {
                    channels.insert(channel.id().to_string(), channel);
                }
            }

            // Fill in server ids, then apply the join-date filter to message-derived kinds
            for item in raw.iter_mut() {
                item.server_id = channels
                    .get(&item.channel_id)
                    .and_then(|c| c.server().map(str::to_string));
            }
            raw.retain(|i| {
                if i.kind == InboxKind::Mention {
                    return true;
                }
                i.server_id
                    .as_ref()
                    .and_then(|s| joined.get(s))
                    .is_some_and(|j| i.message_id >= *j)
            });

            // Load servers and memberships only for servers on this page
            let needed: Vec<String> = raw
                .iter()
                .filter_map(|i| i.server_id.clone())
                .filter(|s| !servers_by_id.contains_key(s))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            if !needed.is_empty() {
                for server in db.fetch_servers(&needed).await? {
                    servers_by_id.insert(server.id.clone(), server);
                }
                for server_id in &needed {
                    if !members_by_server.contains_key(server_id) {
                        if let Ok(member) = db.fetch_member(server_id, &user.id).await {
                            members_by_server.insert(server_id.clone(), member);
                        }
                    }
                }
            }

            // Permission check for channels not decided yet (cached across rounds)
            let to_check: Vec<String> = raw
                .iter()
                .map(|i| i.channel_id.clone())
                .filter(|c| !can_view.contains_key(c))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            for channel_id in to_check {
                let allowed = match channels.get(&channel_id) {
                    // Channel no longer exists
                    None => false,
                    Some(channel) => match channel.server() {
                        // Server channel: reuse the preloaded server and membership.
                        // No membership means the user left, so deny.
                        Some(server_id) => {
                            match (servers_by_id.get(server_id), members_by_server.get(server_id)) {
                                (Some(server), Some(member)) => {
                                    let mut query = DatabasePermissionQuery::new(db, user)
                                        .channel(channel)
                                        .server(server)
                                        .member(member);
                                    calculate_channel_permissions(&mut query)
                                        .await
                                        .has_channel_permission(ChannelPermission::ViewChannel)
                                }
                                _ => false,
                            }
                        }
                        // Group / DM: decided by recipients, no server involved
                        None => {
                            let mut query = DatabasePermissionQuery::new(db, user).channel(channel);
                            calculate_channel_permissions(&mut query)
                                .await
                                .has_channel_permission(ChannelPermission::ViewChannel)
                        }
                    },
                };
                can_view.insert(channel_id, allowed);
            }
            raw.retain(|i| *can_view.get(&i.channel_id).unwrap_or(&false));

            out.extend(raw);

            if out.len() >= limit as usize || exhausted {
                break;
            }
        }

        let next_before = if out.len() >= limit as usize {
            out.truncate(limit as usize);
            out.last().map(|i| i.message_id.clone())
        } else if exhausted {
            None
        } else {
            cursor // hit MAX_ROUNDS with a short page: resume where the scan stopped
        };

        Ok(InboxPage { items: out, next_before })
    }
}

impl InboxItem {
    fn from_message(kind: InboxKind, message: Message, channel_to_server: &HashMap<String, String>) -> Self {
        InboxItem {
            kind,
            message_id: message.id.clone(),
            channel_id: message.channel.clone(),
            server_id: channel_to_server.get(&message.channel).cloned(),
            message: Some(message),
        }
    }
}

impl From<InboxItem> for revolt_models::v0::NotificationCenter {
    fn from(item: InboxItem) -> Self {
        revolt_models::v0::NotificationCenter {
            kind: match item.kind {
                InboxKind::Mention => revolt_models::v0::NotificationKind::Mention,
                InboxKind::Role => revolt_models::v0::NotificationKind::Role,
                InboxKind::Everyone => revolt_models::v0::NotificationKind::Everyone,
            },
            channel_id: item.channel_id,
            message_id: item.message_id,
            server_id: item.server_id,
            message: item.message.expect("filtered above").into_model(None, None),
        }
    }
}
