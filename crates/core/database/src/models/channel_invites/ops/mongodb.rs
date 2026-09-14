use bson::to_bson;
use futures::StreamExt;
use iso8601_timestamp::Timestamp;
use mongodb::options::ReturnDocument;
use revolt_result::Result;

use crate::Invite;
use crate::MongoDb;

use super::AbstractChannelInvites;

static COL: &str = "channel_invites";

#[async_trait]
impl AbstractChannelInvites for MongoDb {
    /// Insert a new invite into the database
    async fn insert_invite(&self, invite: &Invite) -> Result<()> {
        query!(self, insert_one, COL, &invite).map(|_| ())
    }

    /// Fetch an invite by the code
    async fn fetch_invite(&self, code: &str) -> Result<Invite> {
        query!(self, find_one_by_id, COL, code)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all invites for a server
    async fn fetch_invites_for_server(&self, server_id: &str) -> Result<Vec<Invite>> {
        Ok(self
            .col::<Invite>(COL)
            .find(doc! {
                "server": server_id,
            })
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

    /// Delete an invite by its code
    async fn delete_invite(&self, code: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, code).map(|_| ())
    }


    /// Atomically consume one use of an invite, returning the invite's state
    /// *after* the increment — or `None` if it had no uses remaining (or didn't exist).
    async fn consume_invite_use(&self, code: &str) -> Result<Option<Invite>> {
        self.col::<Invite>(COL)
            .find_one_and_update(
                doc! {
                    "_id": code,
                    "$or": [
                        { "max_uses": null },
                        { "$expr": { "$lt": ["$uses", "$max_uses"] } },
                    ],
                },
                doc! { "$inc": { "uses": 1 } },
            )
            .return_document(ReturnDocument::After)
            .await
            .map_err(|_| create_database_error!("find_one_and_update", COL))
    }

    async fn fetch_expired_invites(&self) -> Result<Vec<Invite>> {
        let now = to_bson(&Timestamp::now_utc())
            .map_err(|_| create_database_error!("to_bson", COL))?;

        Ok(self
            .col::<Invite>(COL)
            .find(doc! {
            "expires": { "$lte": now }
        })
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .filter_map(|s| async { s.ok() })
            .collect()
            .await)
    }
}
