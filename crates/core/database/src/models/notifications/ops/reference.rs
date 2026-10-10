use super::AbstractNotifications;
use crate::{NotificationCenter, ReferenceDb};
use revolt_result::Result;

#[async_trait]
impl AbstractNotifications for ReferenceDb {
    async fn insert_notifications(&self, notifications: Vec<NotificationCenter>) -> Result<()> {
        if notifications.is_empty() {
            return Ok(());
        }

        let mut store = self.notifications.lock().await;
        for notification in notifications {
            // Duplicate (user_id, message_id) is ignored, like the Mongo unique index
            store
                .entry((notification.user_id.clone(), notification.message_id.clone()))
                .or_insert(notification);
        }

        Ok(())
    }

    async fn fetch_notifications(
        &self,
        user_id: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NotificationCenter>> {
        let store = self.notifications.lock().await;

        let mut results: Vec<NotificationCenter> = store
            .values()
            .filter(|n| n.user_id == user_id)
            .filter(|n| before.map_or(true, |b| n.message_id.as_str() < b))
            .cloned()
            .collect();

        results.sort_by(|a, b| b.message_id.cmp(&a.message_id));
        results.truncate(limit.max(0) as usize);

        Ok(results)
    }

    async fn delete_notifications_before(&self, before_ms: i64) -> Result<u64> {
        let cutoff = ulid::Ulid::from_parts(before_ms.max(0) as u64, 0).to_string();

        let mut store = self.notifications.lock().await;
        let before_len = store.len();
        store.retain(|_, n| n.message_id.as_str() >= cutoff.as_str());

        Ok((before_len - store.len()) as u64)
    }

    async fn delete_notifications_for_message(&self, message_id: &str) -> Result<()> {
        let mut store = self.notifications.lock().await;
        store.retain(|_, n| n.message_id != message_id);

        Ok(())
    }

    async fn delete_notifications_for_messages(&self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let mut store = self.notifications.lock().await;
        store.retain(|_, n| !message_ids.contains(&n.message_id));

        Ok(())
    }

    async fn delete_notifications_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()> {
        if user_ids.is_empty() {
            return Ok(());
        }

        let mut store = self.notifications.lock().await;
        store.retain(|_, n| !(n.message_id == message_id && user_ids.contains(&n.user_id)));

        Ok(())
    }
}
