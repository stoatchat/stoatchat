use super::AbstractInbox;
use crate::{InboxEntry, InboxFilter, InboxKind, ReferenceDb};
use iso8601_timestamp::Timestamp;
use revolt_result::Result;
use std::collections::HashMap;

#[async_trait]
impl AbstractInbox for ReferenceDb {
    async fn insert_inbox(&self, items: Vec<InboxEntry>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut store = self.inbox.lock().await;
        for item in items {
            // Duplicate (user_id, message_id) is ignored, like the Mongo unique index
            store
                .entry((item.user_id.clone(), item.message_id.clone()))
                .or_insert(item);
        }

        Ok(())
    }

    async fn fetch_inbox_items(
        &self,
        user_id: &str,
        kinds: &[InboxKind],
        created_after: Timestamp,
        channel_id: Option<&str>,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<InboxEntry>> {
        let store = self.inbox.lock().await;

        let mut results: Vec<InboxEntry> = store
            .values()
            .filter(|n| n.user_id == user_id)
            .filter(|n| kinds.contains(&n.entry_kind))
            .filter(|n| n.created_at >= created_after)
            .filter(|n| channel_id.is_none_or(|channel| n.channel_id == channel))
            .filter(|n| before.is_none_or(|b| n.message_id.as_str() < b))
            .cloned()
            .collect();

        results.sort_by(|a, b| b.message_id.cmp(&a.message_id));
        results.truncate(limit.max(0) as usize);

        Ok(results)
    }

    async fn delete_inbox_before(&self, before_ms: i64) -> Result<u64> {
        let cutoff = Timestamp::UNIX_EPOCH
            + iso8601_timestamp::Duration::milliseconds(before_ms.max(0));

        let mut store = self.inbox.lock().await;
        let before_len = store.len();
        store.retain(|_, n| n.created_at >= cutoff);

        Ok((before_len - store.len()) as u64)
    }

    async fn delete_inbox_where(&self, filter: InboxFilter) -> Result<()> {
        let mut store = self.inbox.lock().await;
        store.retain(|_, entry| !filter.matches(entry));

        Ok(())
    }

    async fn trim_inbox(&self, user_ids: Option<&[String]>, max_per_user: usize) -> Result<u64> {
        if max_per_user == 0 {
            return Ok(0);
        }

        let mut store = self.inbox.lock().await;
        let mut by_user: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for entry in store.values() {
            if user_ids.is_none_or(|ids| ids.iter().any(|id| id == &entry.user_id)) {
                by_user
                    .entry(entry.user_id.clone())
                    .or_default()
                    .push((entry.created_at.to_string(), entry.message_id.clone()));
            }
        }

        let mut removed = 0u64;
        for (user_id, mut entries) in by_user {
            if entries.len() <= max_per_user {
                continue;
            }

            entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

            for (_, message_id) in entries.into_iter().skip(max_per_user) {
                if store.remove(&(user_id.clone(), message_id)).is_some() {
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}
