use super::AbstractNotifications;
use crate::{MongoDb, NotificationCenter};
use futures::StreamExt;
use revolt_result::Result;

static COL: &str = "notifications";

#[async_trait]
impl AbstractNotifications for MongoDb {
    async fn insert_notifications(&self, notifications: Vec<NotificationCenter>) -> Result<()> {
        if notifications.is_empty() {
            return Ok(());
        }

        match self
            .col::<NotificationCenter>(COL)
            .insert_many(notifications)
            .ordered(false)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) if is_duplicate_key(&err) => Ok(()),
            Err(_) => Err(create_database_error!("insert_many", COL)),
        }
    }

    async fn fetch_notifications(
        &self,
        user_id: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NotificationCenter>> {
        let mut filter = doc! { "user_id": user_id };
        if let Some(before) = before {
            filter.insert("message_id", doc! { "$lt": before });
        }

        Ok(self
            .col::<NotificationCenter>(COL)
            .find(filter)
            .sort(doc! { "message_id": -1 })
            .limit(limit)
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .filter_map(|s| async {
                if cfg!(debug_assertions) {
                    Some(s.unwrap())
                } else {
                    s.ok()
                }
            })
            .collect()
            .await)
    }

    async fn delete_notifications_before(&self, before_ms: i64) -> Result<u64> {
        let cutoff = ulid::Ulid::from_parts(before_ms.max(0) as u64, 0).to_string();

        self.col::<NotificationCenter>(COL)
            .delete_many(doc! { "_id": { "$lt": cutoff } })
            .await
            .map(|result| result.deleted_count)
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_notifications_for_message(&self, message_id: &str) -> Result<()> {
        self.col::<NotificationCenter>(COL)
            .delete_many(doc! { "message_id": message_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_notifications_for_messages(&self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        self.col::<NotificationCenter>(COL)
            .delete_many(doc! { "message_id": { "$in": message_ids } })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_notifications_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()> {
        if user_ids.is_empty() {
            return Ok(());
        }

        self.col::<NotificationCenter>(COL)
            .delete_many(doc! {
                "message_id": message_id,
                "user_id": { "$in": user_ids },
            })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }
}

fn is_duplicate_key(err: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    match &*err.kind {
        ErrorKind::InsertMany(e) => e
            .write_errors
            .as_ref()
            .map_or(false, |errs| errs.iter().all(|we| we.code == 11000)),
        ErrorKind::Write(WriteFailure::WriteError(we)) => we.code == 11000,
        _ => false,
    }
}
