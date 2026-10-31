use std::time::SystemTime;
use bson::to_bson;
use futures::TryStreamExt;
use ulid::Ulid;
use crate::{InboxEntry, InboxJob, InboxKind, InboxOptions};
use super::AbstractInbox;
use crate::MongoDb;
use revolt_result::Result;

static COL: &str = "inbox";
static JOB_COL: &str = "inbox_jobs";
const DELETE_CHUNK: usize = 1000;

#[async_trait]
impl AbstractInbox for MongoDb {
    async fn insert_inbox_entries(&self, entries: Vec<InboxEntry>) -> Result<()> {
        // Unordered, so one duplicate (a retried send) doesn't abort the rest.
        match self.col::<InboxEntry>(COL).insert_many(entries).ordered(false).await {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("E11000") => Ok(()), // duplicate _id only
            Err(_) => Err(create_database_error!("insert_many", COL)),
        }
    }

    async fn insert_inbox_job(&self, job: &InboxJob) -> Result<()> {
        match self.col::<InboxJob>(JOB_COL).insert_one(job).await {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("E11000") => Ok(()), // job already queued
            Err(_) => Err(create_database_error!("insert_one", JOB_COL)),
        }
    }

    async fn fetch_inbox_jobs(&self, limit: i64) -> Result<Vec<InboxJob>> {
        self.col::<InboxJob>(JOB_COL)
            .find(doc! {})
            .limit(limit)
            .await
            .map_err(|_| create_database_error!("find", JOB_COL))?
            .try_collect()
            .await
            .map_err(|_| create_database_error!("find", JOB_COL))
    }

    async fn delete_inbox_job(&self, message_id: &str) -> Result<()> {
        self.col::<InboxJob>(JOB_COL)
            .delete_one(doc! { "_id": message_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_one", JOB_COL))
    }

    async fn fetch_inbox_entries(&self, user_id: &str, options: &InboxOptions) -> Result<Vec<InboxEntry>> {
        let mut kinds = vec![];
        if options.mentions { kinds.push(InboxKind::Mention); }
        if options.roles { kinds.push(InboxKind::Role); }
        if options.everyone { kinds.push(InboxKind::Everyone); }
        if kinds.is_empty() {
            return Ok(vec![]);
        }

        let mut filter = doc! {
        "user_id": user_id,
        "kind": { "$in": to_bson(&kinds).map_err(|_| create_database_error!("to_bson", COL))? },
        };
        if let Some(before) = &options.before {
            filter.insert("message_id", doc! { "$lt": before });
        }
        if let Some(server_id) = &options.server_id {
            filter.insert("server_id", server_id);
        }

        self.col::<InboxEntry>(COL)
            .find(filter)
            .sort(doc! { "message_id": -1 })
            .limit(options.limit)
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .try_collect()
            .await
            .map_err(|_| create_database_error!("find", COL))
    }

    async fn delete_inbox_entries_for_messages(&self, message_ids: &[String]) -> Result<()> {
        for chunk in message_ids.chunks(DELETE_CHUNK) {
            self.col::<InboxEntry>(COL)
                .delete_many(doc! { "message_id": { "$in": chunk } })
                .await
                .map_err(|_| create_database_error!("delete_many", COL))?;
        }
        Ok(())
    }

    async fn delete_inbox_entries_for_channel(&self, channel_id: &str) -> Result<()> {
        self.col::<InboxEntry>(COL)
            .delete_many(doc! { "channel_id": channel_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_entries_for_server(&self, server_id: &str) -> Result<()> {
        self.col::<InboxEntry>(COL)
            .delete_many(doc! { "server_id": server_id })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn delete_inbox_entries_for_user(
        &self,
        user_id: &str,
        server_id: Option<&str>,
    ) -> Result<()> {
        let mut filter = doc! { "user_id": user_id };
        if let Some(server_id) = server_id {
            filter.insert("server_id", server_id);
        }

        self.col::<InboxEntry>(COL)
            .delete_many(filter)
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }

    async fn prune_inbox_entries(&self, older_than: SystemTime) -> Result<u64> {
        let threshold = Ulid::from_datetime(older_than).to_string();

        self.col::<InboxEntry>(COL)
            .delete_many(doc! { "message_id": { "$lt": threshold } })
            .await
            .map(|result| result.deleted_count)
            .map_err(|_| create_database_error!("delete_many", COL))
    }
}