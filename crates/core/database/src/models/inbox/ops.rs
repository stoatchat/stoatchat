use crate::{InboxEntry, InboxKind};
use revolt_result::Result;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractInbox: Sync + Send {
    /// Insert new inbox entries into the database
    async fn insert_inbox(&self, items: Vec<InboxEntry>) -> Result<()>;

    /// Fetch inbox entries of the given kinds from the database
    async fn fetch_inbox_items(
        &self,
        user_id: &str,
        kinds: &[InboxKind],
        after: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<InboxEntry>>;

    /// Delete inbox entries created before the given time, returns how many were removed
    async fn delete_inbox_before(&self, before_ms: i64) -> Result<u64>;

    /// Delete every inbox entry created for messages
    async fn delete_inbox_for_messages(&self, messages: &[String]) -> Result<()>;

    /// Delete a message's inbox entries for specific users
    async fn delete_inbox_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()>;

    /// Delete a message's inbox entries of one kind (e.g. stored `@online` recipients)
    async fn delete_inbox_for_message_kind(
        &self,
        message_id: &str,
        kind: InboxKind,
    ) -> Result<()>;

    /// Delete every inbox entry in the given channels (channel or server deletion)
    async fn delete_inbox_for_channels(&self, channel_ids: &[String]) -> Result<()>;

    /// Delete every inbox entry addressed to a user (account deletion)
    async fn delete_inbox_for_user(&self, user_id: &str) -> Result<()>;

    /// Delete a user's inbox entries in one channel (leaving or being removed from a group)
    async fn delete_inbox_for_user_in_channel(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<()>;

    /// Delete a user's inbox entries in one server (leaving, kick or ban)
    async fn delete_inbox_for_user_in_server(
        &self,
        user_id: &str,
        server_id: &str,
    ) -> Result<()>;
}
