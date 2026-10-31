use std::time::SystemTime;
use crate::models::inbox::{InboxEntry, InboxJob, InboxOptions};
use revolt_result::Result;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractInbox: Sync + Send {
    async fn insert_inbox_entries(&self, entries: Vec<InboxEntry>) -> Result<()>;
    async fn fetch_inbox_entries(&self, user_id: &str, options: &InboxOptions) -> Result<Vec<InboxEntry>>;

    /// Persist a fan-out job, idempotently.
    async fn insert_inbox_job(&self, job: &InboxJob) -> Result<()>;

    /// Fetch pending fan-out jobs.
    async fn fetch_inbox_jobs(&self, limit: i64) -> Result<Vec<InboxJob>>;

    /// Remove a completed fan-out job.
    async fn delete_inbox_job(&self, message_id: &str) -> Result<()>;

    /// Message deleted (single, bulk, purge)
    async fn delete_inbox_entries_for_messages(&self, message_ids: &[String]) -> Result<()>;

    /// Channel deleted
    async fn delete_inbox_entries_for_channel(&self, channel_id: &str) -> Result<()>;

    /// Server deleted
    async fn delete_inbox_entries_for_server(&self, server_id: &str) -> Result<()>;

    /// User left/was removed from a server (`Some`), or the account was removed (`None`)
    async fn delete_inbox_entries_for_user(
        &self,
        user_id: &str,
        server_id: Option<&str>,
    ) -> Result<()>;

    /// Delete every entry whose message was sent before older_than. Returns how many were removed.
    async fn prune_inbox_entries(&self, older_than: SystemTime) -> Result<u64>;
}