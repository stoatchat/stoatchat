use revolt_result::Result;
use ulid::Ulid;
use std::collections::{HashMap, HashSet};

use iso8601_timestamp::Timestamp;
use revolt_config::config;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};

use crate::util::permissions::DatabasePermissionQuery;
use crate::{Channel, Database, InboxFilter, Member, Message, Server, User};

auto_derived!(
    /// Inbox entry, stored in the database.
    pub struct InboxEntry {
        /// Why this inbox entry exists
        pub entry_kind: InboxKind,
        /// The mentioned user ID
        pub user_id: String,
        /// Message ID
        pub message_id: String,
        /// Channel ID
        pub channel_id: String,
        /// Server ID
        #[serde(skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
        /// When the entry was created; used for retention and trimming
        #[serde(default = "Timestamp::now_utc")]
        pub created_at: Timestamp,
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

    /// Internal inbox item used while assembling a page; carries the resolved
    /// message body. Converted to `v0::InboxEntry` for the API response.
    pub struct InboxItem {
        pub entry_kind: InboxKind,
        pub message_id: String,
        pub channel_id: String,
        pub server_id: Option<String>,
        pub message: Option<Message>,
    }

    pub struct InboxPage {
        pub items: Vec<InboxItem>,
        /// Pass as `before` to continue; None when nothing older exists
        pub pagination: Option<String>,
    }
);

const MAX_ROUNDS: usize = 5;

fn ulid_at(ms: u64) -> String {
    Ulid::from_parts(ms, 0).to_string()
}

fn timestamp_ms(ts: &Timestamp) -> u64 {
    ts.duration_since(Timestamp::UNIX_EPOCH)
        .whole_milliseconds()
        .max(0) as u64
}

pub fn diff_mentions(old: &[String], new: &[String]) -> (Vec<String>, Vec<String>) {
    let old_set: HashSet<&str> = old.iter().map(String::as_str).collect();
    let new_set: HashSet<&str> = new.iter().map(String::as_str).collect();

    let mut added: Vec<String> = new_set
        .difference(&old_set)
        .map(|id| (*id).to_string())
        .collect();
    let mut removed: Vec<String> = old_set
        .difference(&new_set)
        .map(|id| (*id).to_string())
        .collect();

    added.sort_unstable();
    removed.sort_unstable();

    (added, removed)
}

#[allow(clippy::disallowed_methods)]
impl InboxEntry {
    /// Build an inbox entry for one user
    fn new(
        entry_kind: InboxKind,
        user_id: String,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
    ) -> InboxEntry {
        InboxEntry {
            entry_kind,
            user_id,
            message_id: message_id.to_string(),
            channel_id: channel_id.to_string(),
            server_id: server_id.map(str::to_string),
            created_at: Timestamp::now_utc(),
        }
    }

    /// Create inbox entries for everyone in a message's (already permission-filtered) mentions
    pub async fn create_for_mentions(
        db: &Database,
        author_id: &str,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
        mentions: &[String],
    ) -> Result<()> {
        let mut seen = HashSet::new();
        let items: Vec<InboxEntry> = mentions
            .iter()
            .filter(|id| id.as_str() != author_id)
            .filter(|id| seen.insert(id.as_str()))
            .map(|id| InboxEntry::new(InboxKind::Mention, id.clone(), message_id, channel_id, server_id))
            .collect();

        db.insert_inbox(items).await
    }

    /// Sync inbox entries after a message edit: drop removed mentions, add new ones
    pub async fn update_for_edit(
        db: &Database,
        author_id: &str,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
        old_mentions: &[String],
        new_mentions: &[String],
    ) -> Result<()> {
        let (added, removed) = diff_mentions(old_mentions, new_mentions);

        let removed: Vec<String> = removed
            .into_iter()
            .filter(|id| id != author_id)
            .collect();

        db.delete_inbox_where(
            InboxFilter::new()
                .message(message_id)
                .users(&removed),
        )
        .await?;

        InboxEntry::create_for_mentions(db, author_id, message_id, channel_id, server_id, &added)
            .await
    }

    /// Delete all inbox entries for multiple messages
    pub async fn delete_for_messages(db: &Database, message_ids: &[String]) -> Result<()> {
        db.delete_inbox_where(InboxFilter::new().messages(message_ids))
            .await
    }

    pub async fn delete_for_channels(db: &Database, channel_ids: &[String]) -> Result<()> {
        db.delete_inbox_where(InboxFilter::new().channels(channel_ids))
            .await
    }

    pub async fn delete_for_user(db: &Database, user_id: &str) -> Result<()> {
        db.delete_inbox_where(InboxFilter::new().user(user_id)).await
    }

    pub async fn delete_for_user_in_channel(
        db: &Database,
        user_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        db.delete_inbox_where(
            InboxFilter::new()
                .user(user_id)
                .channel(channel_id),
        )
        .await
    }

    pub async fn delete_for_user_in_server(
        db: &Database,
        user_id: &str,
        server_id: &str
    ) -> Result<()> {
        db.delete_inbox_where(
            InboxFilter::new()
                .user(user_id)
                .server(server_id),
        )
        .await
    }

    /// Delete inbox items older than the retention window, returns how many were removed
    pub async fn prune(db: &Database) -> Result<u64> {
        let retention_days = config().await.features.limits.global.inbox_retention_days;


        let mut removed = if retention_days == 0 {
            0
        } else {
            let cutoff =
                timestamp_ms(&Timestamp::now_utc()) as i64 - retention_days as i64 * 86_400_000;
            db.delete_inbox_before(cutoff).await?
        };

        let max_per_user = config().await.features.limits.global.inbox_max_per_user;
        if max_per_user != 0 {
            removed += db.trim_inbox(None, max_per_user).await?;
        }

        Ok(removed)
    }

    pub async fn fetch_inbox(db: &Database, user: &User, opts: InboxOptions) -> Result<InboxPage> {
        let limit = opts.limit.clamp(1, 100);

        // Retention window: older rows are hidden whether or not the prune has run yet
        let days = config().await.features.limits.global.inbox_retention_days;
        let (cutoff_ts, cutoff_id) = if days == 0 {
            (Timestamp::UNIX_EPOCH, ulid_at(0))
        } else {
            let ms = timestamp_ms(&Timestamp::now_utc()) as i64 - days as i64 * 86_400_000;
            let ms = ms.max(0);
            (
                Timestamp::UNIX_EPOCH + iso8601_timestamp::Duration::milliseconds(ms),
                ulid_at(ms as u64),
            )
        };

        // Only loaded when role / everyone mentions are requested
        let mut members_by_server: HashMap<String, Member> = HashMap::new();
        let mut joined: HashMap<String, String> = HashMap::new();
        let mut role_windows: Vec<(String, String)> = vec![];
        let mut channel_to_server: HashMap<String, String> = HashMap::new();
        let mut channel_ids: Vec<String> = vec![];

        if opts.roles || opts.everyone {
            let members = db.fetch_all_memberships(&user.id).await?;

            joined = members
                .iter()
                .map(|m| (m.id.server.clone(), ulid_at(timestamp_ms(&m.joined_at))))
                .collect();

            role_windows = members
                .iter()
                .flat_map(|m| {
                    let joined_ms = timestamp_ms(&m.joined_at);
                    m.roles.iter().map(move |role| {
                        let since_ms = m
                            .role_since(role)
                            .map(|ts| timestamp_ms(&ts))
                            .unwrap_or(joined_ms)
                            .max(joined_ms);
                        (role.clone(), ulid_at(since_ms))
                    })
                })
                .collect();

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

            // Only direct mentions are persisted; role / `@everyone` entries are
            // derived from the message itself when a page is assembled.
            let mut kinds: Vec<InboxKind> = vec![];
            if opts.mentions {
                kinds.push(InboxKind::Mention);
            }
            if !kinds.is_empty() {
                for n in db
                    .fetch_inbox_items(&user.id, &kinds, cutoff_ts, None, before, limit)
                    .await?
                {
                    raw.push(InboxItem {
                        entry_kind: n.entry_kind,
                        message_id: n.message_id,
                        channel_id: n.channel_id,
                        server_id: n.server_id,
                        message: None,
                    });
                }
            }
            if opts.roles {
                for m in db
                    .fetch_role_mention_messages(&role_windows, &user.id, &cutoff_id, before, limit)
                    .await?
                {
                    raw.push(InboxItem::from_message(InboxKind::Role, m, &channel_to_server));
                }
            }
            if opts.everyone {
                let mut all = db
                    .fetch_everyone_mention_messages(
                        &channel_ids,
                        &user.id,
                        &cutoff_id,
                        before,
                        limit,
                    )
                    .await?;
                all.sort_by(|a, b| b.id.cmp(&a.id));
                all.truncate(limit as usize);

                for m in all {
                    raw.push(InboxItem::from_message(InboxKind::Everyone, m, &channel_to_server));
                }
            }

            // Newest first, one entry per message (Mention beats Role beats Everyone)
            raw.sort_by(|a, b| b.message_id.cmp(&a.message_id).then(a.entry_kind.cmp(&b.entry_kind)));
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
                    i.entry_kind == InboxKind::Mention || !message.has_suppressed_notifications()
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
                if i.entry_kind == InboxKind::Mention {
                    return i
                        .message
                        .as_ref()
                        .and_then(|m| m.mentions.as_ref())
                        .is_some_and(|mentions| mentions.contains(&user.id));
                }
                if i.entry_kind == InboxKind::Role {
                    return true;
                }
                let anchor = i
                    .message
                    .as_ref()
                    .and_then(|m| m.everyone_mention_id.as_deref())
                    .unwrap_or(i.message_id.as_str());
                i.server_id
                    .as_ref()
                    .and_then(|s| joined.get(s))
                    .is_some_and(|j| anchor >= j.as_str())
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
                                    let permissions = calculate_channel_permissions(&mut query).await;
                                    permissions
                                        .has_channel_permission(ChannelPermission::ViewChannel)
                                        && permissions.has_channel_permission(
                                            ChannelPermission::ReadMessageHistory,
                                        )
                                }
                                _ => false,
                            }
                        }
                        // Group / DM: decided by recipients, no server involved
                        None => {
                            let mut query = DatabasePermissionQuery::new(db, user).channel(channel);
                            let permissions = calculate_channel_permissions(&mut query).await;
                            permissions.has_channel_permission(ChannelPermission::ViewChannel)
                                && permissions
                                    .has_channel_permission(ChannelPermission::ReadMessageHistory)
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

        let pagination = if out.len() > limit as usize {
            // More candidates were scanned than fit in the page: the remaining
            // ones are older than the page boundary.
            out.truncate(limit as usize);
            out.last().map(|i| i.message_id.clone())
        } else if out.len() == limit as usize && !exhausted {
            // Full page and at least one source may have more.
            out.last().map(|i| i.message_id.clone())
        } else if exhausted {
            None
        } else {
            cursor // hit MAX_ROUNDS with a short page: resume where the scan stopped
        };

        Ok(InboxPage { items: out, pagination })
    }
}

impl InboxItem {
    fn from_message(
        entry_kind: InboxKind,
        message: Message,
        channel_to_server: &HashMap<String, String>,
    ) -> Self {
        InboxItem {
            entry_kind,
            message_id: message.id.clone(),
            channel_id: message.channel.clone(),
            server_id: channel_to_server.get(&message.channel).cloned(),
            message: Some(message),
        }
    }
}

impl From<InboxItem> for revolt_models::v0::InboxEntry {
    fn from(item: InboxItem) -> Self {
        revolt_models::v0::InboxEntry {
            entry_kind: match item.entry_kind {
                InboxKind::Mention => revolt_models::v0::InboxKind::Mention,
                InboxKind::Role => revolt_models::v0::InboxKind::Role,
                InboxKind::Everyone => revolt_models::v0::InboxKind::Everyone,
            },
            channel_id: item.channel_id,
            message_id: item.message_id,
            server_id: item.server_id,
            message: item.message.expect("filtered above").into_model(None, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::diff_mentions;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn diff_reports_added_and_removed() {
        let (added, removed) = diff_mentions(&ids(&["a", "b"]), &ids(&["b", "c"]));

        assert_eq!(added, ids(&["c"]));
        assert_eq!(removed, ids(&["a"]));
    }

    #[test]
    fn diff_deduplicates_and_sorts_both_sides() {
        let (added, removed) = diff_mentions(
            &ids(&["b", "a", "a", "same"]),
            &ids(&["same", "c", "b", "b"]),
        );

        assert_eq!(added, ids(&["c"]));
        assert_eq!(removed, ids(&["a"]));
    }

    #[test]
    fn diff_is_empty_for_equal_sets() {
        let (added, removed) = diff_mentions(&ids(&["a", "b", "b"]), &ids(&["b", "a"]));

        assert!(added.is_empty());
        assert!(removed.is_empty());
    }
}
