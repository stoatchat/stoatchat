use super::AbstractInbox;
use crate::{MongoDb, Inbox};
use futures::StreamExt;
use revolt_result::Result;

static COL: &str = "inbox";

#[async_trait]
impl AbstractInbox for MongoDb {
    async fn insert_inbox(&self, items: Vec<Inbox>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        match self
            .col::<Inbox>(COL)
            .insert_many(items)
            .ordered(false)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) if is_duplicate_key(&err) => Ok(()),
            Err(_) => Err(create_database_error!("insert_many", COL)),
        }
    }

    async fn fetch_inbox_items(
        &self,
        user_id: &str,
        after: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Inbox>> {
        let mut id_range = doc! { "$gt": after };
        if let Some(before) = before {
            id_range.insert("$lt", before);
        }
        let filter = doc! { "user_id": user_id, "message_id": id_range };

        Ok(self
            .col::<Inbox>(COL)
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

    async fn delete_inbox_before(&self, before_ms: i64) -> Result<u64> {
        let cutoff = ulid::Ulid::from_parts(before_ms.max(0) as u64, 0).to_string();

        self.col::<Inbox>(COL)
            .delete_many(doc! { "message_id": { "$lt": cutoff } })
            .await
            .map(|result| result.deleted_count)
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_message(&self, message_id: &str) -> Result<()> {
        self.col::<Inbox>(COL)
            .delete_many(doc! { "message_id": message_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_messages(&self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        self.col::<Inbox>(COL)
            .delete_many(doc! { "message_id": { "$in": message_ids } })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_users(
        &self,
        message_id: &str,
        user_ids: &[String],
    ) -> Result<()> {
        if user_ids.is_empty() {
            return Ok(());
        }

        self.col::<Inbox>(COL)
            .delete_many(doc! {
                "message_id": message_id,
                "user_id": { "$in": user_ids },
            })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_channels(&self, channel_ids: &[String]) -> Result<()> {
        if channel_ids.is_empty() {
            return Ok(());
        }

        self.col::<Inbox>(COL)
            .delete_many(doc! { "channel_id": { "$in": channel_ids } })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_user(&self, user_id: &str) -> Result<()> {
        self.col::<Inbox>(COL)
            .delete_many(doc! { "user_id": user_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_user_in_channel(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        self.col::<Inbox>(COL)
            .delete_many(doc! { "user_id": user_id, "channel_id": channel_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_user_in_server(
        &self,
        user_id: &str,
        server_id: &str,
    ) -> Result<()> {
        self.col::<Inbox>(COL)
            .delete_many(doc! { "user_id": user_id, "server_id": server_id })
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
