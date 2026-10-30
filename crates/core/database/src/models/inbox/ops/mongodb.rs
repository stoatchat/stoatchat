use super::AbstractInbox;
use crate::{InboxEntry, InboxFilter, InboxKind, MongoDb, util::mongodb::is_duplicate_key};
use bson::{Document, to_bson};
use futures::StreamExt;
use iso8601_timestamp::Timestamp;
use revolt_result::Result;

static COL: &str = "inbox";

/// Maximum number of ids sent in one `$in` clause.
const MAX_IN: usize = 10_000;

fn filter_documents(filter: &InboxFilter) -> Vec<Document> {
    let mut base = Document::new();
    if let Some(user_id) = &filter.user_id {
        base.insert("user_id", user_id);
    }
    if let Some(message_id) = &filter.message_id {
        base.insert("message_id", message_id);
    }
    if let Some(channel_id) = &filter.channel_id {
        base.insert("channel_id", channel_id);
    }
    if let Some(server_id) = &filter.server_id {
        base.insert("server_id", server_id);
    }

    let mut lists: Vec<(&str, Vec<String>)> = vec![];
    if let Some(user_ids) = &filter.user_ids {
        lists.push(("user_id", user_ids.clone()));
    }
    if let Some(message_ids) = &filter.message_ids {
        lists.push(("message_id", message_ids.clone()));
    }
    if let Some(channel_ids) = &filter.channel_ids {
        lists.push(("channel_id", channel_ids.clone()));
    }

    let Some((chunk_field, chunk_values)) = lists.first().cloned() else {
        return vec![base];
    };

    for (field, values) in lists.into_iter().skip(1) {
        base.insert(field, doc! { "$in": values });
    }

    chunk_values
        .chunks(MAX_IN)
        .map(|chunk| {
            let mut filter = base.clone();
            filter.insert(chunk_field, doc! { "$in": chunk });
            filter
        })
        .collect()
}

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
        created_after: Timestamp,
        channel_id: Option<&str>,
        before: Option<&str>,
        limit: i64,
    ) -> Result<Vec<InboxEntry>> {
        if kinds.is_empty() {
            return Ok(vec![]);
        }

        let cutoff =
            to_bson(&created_after).map_err(|_| create_database_error!("to_bson", COL))?;
        let kinds = to_bson(kinds).map_err(|_| create_database_error!("to_bson", COL))?;
        let mut filter = doc! {
            "user_id": user_id,
            "entry_kind": { "$in": kinds },
            "created_at": { "$gte": cutoff },
        };
        if let Some(channel_id) = channel_id {
            filter.insert("channel_id", channel_id);
        }
        if let Some(before) = before {
            filter.insert("message_id", doc! { "$lt": before });
        }

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

        let cutoff = to_bson(&(Timestamp::UNIX_EPOCH
            + iso8601_timestamp::Duration::milliseconds(before_ms.max(0))))
        .map_err(|_| create_database_error!("to_bson", COL))?;
        let mut deleted = 0u64;

        loop {
            let ids: Vec<bson::Bson> = self
                .col::<bson::Document>(COL)
                .find(doc! { "created_at": { "$lt": &cutoff } })
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

    async fn delete_inbox_where(&self, filter: InboxFilter) -> Result<()> {
        for document in filter_documents(&filter) {
            self.col::<Document>(COL)
                .delete_many(document)
                .await
                .map_err(|_| create_database_error!("delete_many", COL))?;
        }

        Ok(())
    }

    async fn trim_inbox(&self, user_ids: Option<&[String]>, max_per_user: usize) -> Result<u64> {
        if max_per_user == 0 {
            return Ok(0);
        }

        let mut pipeline = vec![];
        if let Some(user_ids) = user_ids {
            if user_ids.is_empty() {
                return Ok(0);
            }
            pipeline.push(doc! { "$match": { "user_id": { "$in": user_ids } } });
        }

        pipeline.extend([
            doc! { "$sort": { "user_id": 1, "created_at": -1, "message_id": -1 } },
            doc! { "$group": { "_id": "$user_id", "ids": { "$push": "$_id" }, "count": { "$sum": 1 } } },
            doc! { "$match": { "count": { "$gt": max_per_user as i64 } } },
            doc! { "$project": { "overflow": { "$slice": [
                "$ids",
                max_per_user as i64,
                { "$subtract": ["$count", max_per_user as i64] }
            ] } } },
        ]);

        let mut cursor = self
            .col::<Document>(COL)
            .aggregate(pipeline)
            .allow_disk_use(true)
            .await
            .map_err(|_| create_database_error!("aggregate", COL))?;

        let mut overflow = vec![];
        while let Some(document) = cursor.next().await {
            let document = document.map_err(|_| create_database_error!("aggregate", COL))?;
            if let Ok(ids) = document.get_array("overflow") {
                overflow.extend(ids.iter().cloned());
            }
        }

        let mut deleted = 0u64;
        for chunk in overflow.chunks(MAX_IN) {
            deleted += self
                .col::<Document>(COL)
                .delete_many(doc! { "_id": { "$in": chunk } })
                .await
                .map_err(|_| create_database_error!("delete_many", COL))?
                .deleted_count;
        }

        Ok(deleted)
    }
}
