use revolt_result::Result;
use ulid::Ulid;
use std::collections::HashMap;

use iso8601_timestamp::Timestamp;
use revolt_config::config;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};

use crate::util::permissions::DatabasePermissionQuery;
use crate::{Database, Message, User};

auto_derived!(
    /// Notification hub
    pub struct Notification {
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
);

fn ulid_at(ms: u64) -> String {
    Ulid::from_parts(ms, 0).to_string()
}

#[allow(clippy::disallowed_methods)]
impl Notification {
    /// Build a notification for one user
    fn new(
        kind: InboxKind,
        user_id: String,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
    ) -> Notification {
        Notification {
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
        let notifications: Vec<Notification> = mentions
            .iter()
            .filter(|id| id.as_str() != author_id)
            .map(|id| Notification::new(InboxKind::Mention, id.clone(), message_id, channel_id, server_id))
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
        Notification::create_for_mentions(db, author_id, message_id, channel_id, server_id, &added)
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

    /// Fetch a user's notifications, newest first
    pub async fn fetch(
        db: &Database,
        user_id: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Notification>> {
        db.fetch_notifications(user_id, before, limit).await
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

    pub async fn fetch_inbox(
            db: &Database,
            user: &User,
            opts: InboxOptions,
        ) -> Result<Vec<InboxItem>> {
            let limit = opts.limit;
            let before = opts.before.as_deref();
            let mut items: Vec<InboxItem> = vec![];

            if opts.mentions {
                for n in db.fetch_notifications(&user.id, before, limit).await? {
                    items.push(InboxItem {
                        kind: InboxKind::Mention,
                        message_id: n.message_id,
                        channel_id: n.channel_id,
                        server_id: n.server_id,
                        message: None,
                    });
                }
            }

            if opts.roles || opts.everyone {
                let members = db.fetch_all_memberships(&user.id).await?;

                let joined: HashMap<String, String> = members
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

                let role_ids: Vec<String> = members.iter().flat_map(|m| m.roles.clone()).collect();
                let server_ids: Vec<String> = members.iter().map(|m| m.id.server.clone()).collect();

                let mut channel_to_server: HashMap<String, String> = HashMap::new();
                for server in db.fetch_servers(&server_ids).await? {
                    for channel in &server.channels {
                        channel_to_server.insert(channel.clone(), server.id.clone());
                    }
                }
                let channel_ids: Vec<String> = channel_to_server.keys().cloned().collect();

                let days = config().await.features.limits.global.notification_retention_days;
                let cutoff = if days == 0 {
                    ulid_at(0)
                } else {
                    let ms = bson::DateTime::now().timestamp_millis() - days as i64 * 86_400_000;
                    ulid_at(ms.max(0) as u64)
                };

                let mut found: Vec<(InboxKind, Message)> = vec![];
                if opts.roles {
                    for m in db
                        .fetch_role_mention_messages(&channel_ids, &role_ids, &user.id, &cutoff, before, limit)
                        .await?
                    {
                        found.push((InboxKind::Role, m));
                    }
                }
                if opts.everyone {
                    for m in db
                        .fetch_everyone_mention_messages(&channel_ids, &user.id, &cutoff, before, limit)
                        .await?
                    {
                        found.push((InboxKind::Everyone, m));
                    }
                }

                for (kind, message) in found {
                    let server_id = channel_to_server.get(&message.channel).cloned();

                    // Skip silent messages and anything from before they joined
                    let joined_ok = server_id
                        .as_ref()
                        .and_then(|s| joined.get(s))
                        .is_some_and(|j| message.id >= *j);

                    if message.has_suppressed_notifications() || !joined_ok {
                        continue;
                    }

                    items.push(InboxItem {
                        kind,
                        message_id: message.id.clone(),
                        channel_id: message.channel.clone(),
                        server_id,
                        message: Some(message),
                    });
                }
            }

            items.sort_by(|a, b| {
                b.message_id
                    .cmp(&a.message_id)
                    .then(a.kind.cmp(&b.kind))
            });
            items.dedup_by(|a, b| a.message_id == b.message_id);

            let channel_ids: Vec<String> = items
                .iter()
                .map(|i| i.channel_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let mut can_view: HashMap<String, bool> = HashMap::new();
            for channel in db.fetch_channels(&channel_ids).await? {
                let mut query = DatabasePermissionQuery::new(db, user).channel(&channel);
                let allowed = calculate_channel_permissions(&mut query)
                    .await
                    .has_channel_permission(ChannelPermission::ViewChannel);
                can_view.insert(channel.id().to_string(), allowed);
            }
            items.retain(|i| *can_view.get(&i.channel_id).unwrap_or(&false));

            items.truncate(limit as usize);

            let missing: Vec<String> = items
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

                for item in items.iter_mut().filter(|i| i.message.is_none()) {
                    item.message = fetched.remove(&item.message_id);
                }
            }
            items.retain(|i| i.message.is_some());

            Ok(items)
        }
}

impl From<InboxItem> for revolt_models::v0::Notification {
    fn from(item: InboxItem) -> Self {
        revolt_models::v0::Notification {
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
