use super::AbstractNotifications;
use crate::{NotificationCenter, ReferenceDb};
use revolt_result::Result;

#[async_trait]
impl AbstractNotifications for ReferenceDb {
    async fn insert_notifications(&self, notifications: Vec<NotificationCenter>) -> Result<()> {
        todo!()
    }

    async fn fetch_notifications(
        &self,
        user_id: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NotificationCenter>> {
        todo!()
    }

    async fn delete_notifications_before(&self, before_ms: i64) -> Result<u64> {
        todo!()
    }

    async fn delete_notifications_for_message(&self, message_id: &str) -> Result<()> {
        todo!()
    }
    async fn delete_notifications_for_messages(&self, message_ids: &[String]) -> Result<()> {
        todo!()
    }

    async fn delete_notifications_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()> {
        todo!()
    }
}
