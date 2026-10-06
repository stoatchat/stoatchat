use revolt_result::Result;
use ulid::Ulid;

use crate::Database;

auto_derived!(
    /// Notification hub
    pub struct Notification {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
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
);

#[allow(clippy::disallowed_methods)]
impl Notification {
    /// Build a notification for one user
    fn new(
        user_id: String,
        message_id: &str,
        channel_id: &str,
        server_id: Option<&str>,
    ) -> Notification {
        Notification {
            id: Ulid::new().to_string(),
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
            .map(|id| Notification::new(id.clone(), message_id, channel_id, server_id))
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
    pub async fn prune(db: &Database, retention_days: u32) -> Result<u64> {
        if retention_days == 0 {
            return Ok(0);
        }

        let cutoff =
            bson::DateTime::now().timestamp_millis() - retention_days as i64 * 86_400_000;
        db.delete_notifications_before(cutoff).await
    }
}

impl From<Notification> for revolt_models::v0::Notification {
    fn from(n: Notification) -> Self {
        revolt_models::v0::Notification {
            id: n.id,
            message_id: n.message_id,
            channel_id: n.channel_id,
            server_id: n.server_id,
        }
    }
}
