use crate::Inbox;
use revolt_result::Result;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractInbox: Sync + Send {
    /// Insert new inbox items into the database
    async fn insert_inbox(&self, items: Vec<Inbox>) -> Result<()>;

    /// Fetch inbox items from the database
    async fn fetch_inbox_items(
        &self,
        user_id: &str,
        after: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Inbox>>;

    /// Delete inbox items created before the given time, returns how many were removed
    async fn delete_inbox_before(&self, before_ms: i64) -> Result<u64>;

    /// Delete every inbox item created for a message
    async fn delete_inbox_for_message(&self, message_id: &str) -> Result<()>;

    /// Delete every inbox item created for messages
    async fn delete_inbox_for_messages(&self, messages: &[String]) -> Result<()>;

    /// Delete a message's inbox items for specific users
    async fn delete_inbox_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()>;

    /// Delete every inbox item in the given channels (channel or server deletion)
    async fn delete_inbox_for_channels(&self, channel_ids: &[String]) -> Result<()>;

    /// Delete every inbox item addressed to a user (account deletion)
    async fn delete_inbox_for_user(&self, user_id: &str) -> Result<()>;

    /// Delete a user's inbox items in one channel (leaving or being removed from a group)
    async fn delete_inbox_for_user_in_channel(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<()>;

    /// Delete a user's inbox items in one server (leaving, kick or ban)
    async fn delete_inbox_for_user_in_server(
        &self,
        user_id: &str,
        server_id: &str,
    ) -> Result<()>;
}
