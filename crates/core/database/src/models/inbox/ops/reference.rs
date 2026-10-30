use std::time::SystemTime;

use revolt_result::Result;
use ulid::Ulid;

use super::AbstractInbox;
use crate::{InboxEntry, InboxJob, InboxKind, InboxOptions};
use crate::ReferenceDb;

#[async_trait]
impl AbstractInbox for ReferenceDb {
    async fn insert_inbox_entries(&self, entries: Vec<InboxEntry>) -> Result<()> {
        let mut inbox = self.inbox.lock().await;
        for entry in entries {
            // Like a unique `_id` in Mongo: a duplicate insert is a no-op.
            inbox.entry(entry.id.clone()).or_insert(entry);
        }
        Ok(())
    }

    async fn insert_inbox_job(&self, job: &InboxJob) -> Result<()> {
        self.inbox_jobs
            .lock()
            .await
            .entry(job.message_id.clone())
            .or_insert_with(|| job.clone());
        Ok(())
    }

    async fn fetch_inbox_jobs(&self, limit: i64) -> Result<Vec<InboxJob>> {
        Ok(self
            .inbox_jobs
            .lock()
            .await
            .values()
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }

    async fn delete_inbox_job(&self, message_id: &str) -> Result<()> {
        self.inbox_jobs.lock().await.remove(message_id);
        Ok(())
    }

    async fn fetch_inbox_entries(
        &self,
        user_id: &str,
        options: &InboxOptions,
    ) -> Result<Vec<InboxEntry>> {
        let inbox = self.inbox.lock().await;

        let mut entries: Vec<InboxEntry> = inbox
            .values()
            .filter(|e| e.user_id == user_id)
            .filter(|e| match e.kind {
                InboxKind::Mention => options.mentions,
                InboxKind::Role => options.roles,
                InboxKind::Everyone => options.everyone,
            })
            .filter(|e| {
                options
                    .before
                    .as_ref()
                    .is_none_or(|before| e.message_id.as_str() < before.as_str())
            })
            .filter(|e| {
                options
                    .server_id
                    .as_ref()
                    .is_none_or(|server| e.server_id.as_ref() == Some(server))
            })
            .cloned()
            .collect();

        entries.sort_by(|a, b| b.message_id.cmp(&a.message_id));
        entries.truncate(options.limit.max(0) as usize);
        Ok(entries)
    }

    async fn delete_inbox_entries_for_messages(&self, message_ids: &[String]) -> Result<()> {
        self.inbox
            .lock()
            .await
            .retain(|_, e| !message_ids.contains(&e.message_id));
        Ok(())
    }

    async fn delete_inbox_entries_for_channel(&self, channel_id: &str) -> Result<()> {
        self.inbox
            .lock()
            .await
            .retain(|_, e| e.channel_id != channel_id);
        Ok(())
    }

    async fn delete_inbox_entries_for_server(&self, server_id: &str) -> Result<()> {
        self.inbox
            .lock()
            .await
            .retain(|_, e| e.server_id.as_deref() != Some(server_id));
        Ok(())
    }

    async fn delete_inbox_entries_for_user(
        &self,
        user_id: &str,
        server_id: Option<&str>,
    ) -> Result<()> {
        self.inbox.lock().await.retain(|_, e| {
            let matches_user = e.user_id == user_id;
            let matches_server = server_id.is_none_or(|s| e.server_id.as_deref() == Some(s));
            !(matches_user && matches_server)
        });
        Ok(())
    }

    async fn prune_inbox_entries(&self, older_than: SystemTime) -> Result<u64> {
        let threshold = Ulid::from_datetime(older_than).to_string();

        let mut inbox = self.inbox.lock().await;
        let before = inbox.len();
        inbox.retain(|_, e| e.message_id >= threshold);
        Ok((before - inbox.len()) as u64)
    }
}