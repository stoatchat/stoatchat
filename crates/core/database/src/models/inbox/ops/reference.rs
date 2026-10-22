use super::AbstractInbox;
use crate::{Inbox, ReferenceDb};
use revolt_result::Result;

#[async_trait]
impl AbstractInbox for ReferenceDb {
    async fn insert_inbox(&self, items: Vec<Inbox>) -> Result<()> {
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
        after: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Inbox>> {
        let store = self.inbox.lock().await;

        let mut results: Vec<Inbox> = store
            .values()
            .filter(|n| n.user_id == user_id)
            .filter(|n| n.message_id.as_str() > after)
            .filter(|n| before.map_or(true, |b| n.message_id.as_str() < b))
            .cloned()
            .collect();

        results.sort_by(|a, b| b.message_id.cmp(&a.message_id));
        results.truncate(limit.max(0) as usize);

        Ok(results)
    }

    async fn delete_inbox_before(&self, before_ms: i64) -> Result<u64> {
        let cutoff = ulid::Ulid::from_parts(before_ms.max(0) as u64, 0).to_string();

        let mut store = self.inbox.lock().await;
        let before_len = store.len();
        store.retain(|_, n| n.message_id.as_str() >= cutoff.as_str());

        Ok((before_len - store.len()) as u64)
    }

    async fn delete_inbox_for_message(&self, message_id: &str) -> Result<()> {
        let mut store = self.inbox.lock().await;
        store.retain(|_, n| n.message_id != message_id);

        Ok(())
    }

    async fn delete_inbox_for_messages(&self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let mut store = self.inbox.lock().await;
        store.retain(|_, n| !message_ids.contains(&n.message_id));

        Ok(())
    }

    async fn delete_inbox_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()> {
        if user_ids.is_empty() {
            return Ok(());
        }

        let mut store = self.inbox.lock().await;
        store.retain(|_, n| !(n.message_id == message_id && user_ids.contains(&n.user_id)));

        Ok(())
    }

    async fn delete_inbox_for_channels(&self, channel_ids: &[String]) -> Result<()> {
        if channel_ids.is_empty() {
            return Ok(());
        }

        let mut store = self.inbox.lock().await;
        store.retain(|_, n| !channel_ids.contains(&n.channel_id));

        Ok(())
    }

    async fn delete_inbox_for_user(&self, user_id: &str) -> Result<()> {
        let mut store = self.inbox.lock().await;
        store.retain(|_, n| n.user_id != user_id);

        Ok(())
    }

    async fn delete_inbox_for_user_in_channel(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let mut store = self.inbox.lock().await;
        store.retain(|_, n| !(n.user_id == user_id && n.channel_id == channel_id));

        Ok(())
    }

    async fn delete_inbox_for_user_in_server(
        &self,
        user_id: &str,
        server_id: &str,
    ) -> Result<()> {
        let mut store = self.inbox.lock().await;
        store.retain(|_, n| {
            !(n.user_id == user_id && n.server_id.as_deref() == Some(server_id))
        });

        Ok(())
    }
}
