use crate::Notification;
use revolt_result::Result;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractNotifications: Sync + Send {
    /// Insert a new notification to the database
    async fn insert_notifications(&self, notifications: Vec<Notification>) -> Result<()>;

    /// Fetch notifications from the database
    async fn fetch_notifications(
        &self,
        user_id: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Notification>>;

    /// Delete notifications created before the given time, returns how many were removed
    async fn delete_notifications_before(&self, before_ms: i64) -> Result<u64>;

    /// Delete every notification created for a message
    async fn delete_notifications_for_message(&self, message_id: &str) -> Result<()>;

    /// Delete every notification created for messages
    async fn delete_notifications_for_messages(&self, messages: &[String]) -> Result<()>;

    /// Delete a message's notifications for specific users
    async fn delete_notifications_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()>;
}
