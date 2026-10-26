use super::AbstractInbox;
use crate::{util::mongodb::is_duplicate_key, InboxEntry, InboxKind, MongoDb};
use bson::to_bson;
use futures::StreamExt;
use revolt_result::Result;

static COL: &str = "inbox";

#[async_trait]
impl AbstractInbox for MongoDb {
    async fn insert_inbox(&self, items: Vec<InboxEntry>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        match self
            .col::<InboxEntry>(COL)
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
        kinds: &[InboxKind],
        after: &str,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<InboxEntry>> {
        if kinds.is_empty() {
            return Ok(vec![]);
        }

        let mut id_range = doc! { "$gt": after };
        if let Some(before) = before {
            id_range.insert("$lt", before);
        }
        let kinds = to_bson(kinds).map_err(|_| create_database_error!("to_bson", COL))?;
        let filter = doc! {
            "user_id": user_id,
            "entry_kind": { "$in": kinds },
            "message_id": id_range,
        };

        Ok(self
            .col::<InboxEntry>(COL)
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
        // Delete in bounded batches so the hourly prune never issues one
        // unbounded delete against a large collection.
        const BATCH: i64 = 1_000;

        let cutoff = ulid::Ulid::from_parts(before_ms.max(0) as u64, 0).to_string();
        let mut deleted = 0u64;

        loop {
            let ids: Vec<bson::Bson> = self
                .col::<bson::Document>(COL)
                .find(doc! { "message_id": { "$lt": &cutoff } })
                .projection(doc! { "_id": 1_i32 })
                .limit(BATCH)
                .await
                .map_err(|_| create_database_error!("find", COL))?
                .filter_map(|doc| async move { doc.ok().and_then(|d| d.get("_id").cloned()) })
                .collect()
                .await;

            if ids.is_empty() {
                break;
            }

            let count = ids.len();

            deleted += self
                .col::<bson::Document>(COL)
                .delete_many(doc! { "_id": { "$in": ids } })
                .await
                .map_err(|_| create_database_error!("delete_many", COL))?
                .deleted_count;

            if count < BATCH as usize {
                break;
            }
        }

        Ok(deleted)
    }

    async fn delete_inbox_for_messages(&self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        self.col::<InboxEntry>(COL)
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

        self.col::<InboxEntry>(COL)
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

        self.col::<InboxEntry>(COL)
            .delete_many(doc! { "channel_id": { "$in": channel_ids } })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_for_user(&self, user_id: &str) -> Result<()> {
        self.col::<InboxEntry>(COL)
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
        self.col::<InboxEntry>(COL)
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
        self.col::<InboxEntry>(COL)
            .delete_many(doc! { "user_id": user_id, "server_id": server_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }
}
