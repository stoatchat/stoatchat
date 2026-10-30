use crate::{InboxEntry, InboxKind};
use iso8601_timestamp::Timestamp;
use revolt_result::Result;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[derive(Debug, Default, Clone)]
pub struct InboxFilter {
    pub user_id: Option<String>,
    pub user_ids: Option<Vec<String>>,
    pub message_id: Option<String>,
    pub message_ids: Option<Vec<String>>,
    pub channel_id: Option<String>,
    pub channel_ids: Option<Vec<String>>,
    pub server_id: Option<String>,
}

impl InboxFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn users(mut self, user_ids: &[String]) -> Self {
        self.user_ids = Some(user_ids.to_vec());
        self
    }

    pub fn message(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    pub fn messages(mut self, message_ids: &[String]) -> Self {
        self.message_ids = Some(message_ids.to_vec());
        self
    }

    pub fn channel(mut self, channel_id: impl Into<String>) -> Self {
        self.channel_id = Some(channel_id.into());
        self
    }

    pub fn channels(mut self, channel_ids: &[String]) -> Self {
        self.channel_ids = Some(channel_ids.to_vec());
        self
    }

    pub fn server(mut self, server_id: impl Into<String>) -> Self {
        self.server_id = Some(server_id.into());
        self
    }

    fn matches(&self, entry: &InboxEntry) -> bool {
        let matches_string = |expected: &Option<String>, actual: &str| {
            expected.as_deref().is_none_or(|value| value == actual)
        };
        let matches_list = |expected: &Option<Vec<String>>, actual: &str| {
            expected
                .as_deref()
                .is_none_or(|values| values.iter().any(|value| value == actual))
        };

        matches_string(&self.user_id, &entry.user_id)
            && matches_list(&self.user_ids, &entry.user_id)
            && matches_string(&self.message_id, &entry.message_id)
            && matches_list(&self.message_ids, &entry.message_id)
            && matches_string(&self.channel_id, &entry.channel_id)
            && matches_list(&self.channel_ids, &entry.channel_id)
            && matches_string(
                &self.server_id,
                entry.server_id.as_deref().unwrap_or_default(),
            )
    }
}

#[async_trait]
pub trait AbstractInbox: Sync + Send {
    /// Insert new inbox entries into the database
    async fn insert_inbox(&self, items: Vec<InboxEntry>) -> Result<()>;

    /// Fetch inbox entries of the given kinds created at or after `created_after`
    async fn fetch_inbox_items(
        &self,
        user_id: &str,
        kinds: &[InboxKind],
        created_after: Timestamp,
        channel_id: Option<&str>,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<InboxEntry>>;

    /// Delete inbox entries created before the given time, returns how many were removed
    async fn delete_inbox_before(&self, before_ms: i64) -> Result<u64>;

    /// Delete every inbox entry matching `filter`
    async fn delete_inbox_where(&self, filter: InboxFilter) -> Result<()>;

    /// Keep at most `max_per_user` entries for each user, newest first.
    ///
    /// Passing `None` trims every user and is used by the periodic prune.
    async fn trim_inbox(&self, user_ids: Option<&[String]>, max_per_user: usize) -> Result<u64>;
}
