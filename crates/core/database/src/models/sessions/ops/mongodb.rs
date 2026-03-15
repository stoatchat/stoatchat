use crate::{AbstractSessions, MongoDb, Session};
use bson::{to_bson, to_document};
use futures::TryStreamExt;
use iso8601_timestamp::Timestamp;
use mongodb::options::UpdateOptions;
use revolt_result::Result;

const COL: &str = "sessions";

#[async_trait]
impl AbstractSessions for MongoDb {
    /// Find session by id
    async fn fetch_session(&self, id: &str) -> Result<Session> {
        self.col(COL)
            .find_one(doc! {
                "_id": id
            })
            .await
            .map_err(|_| create_database_error!("find_one", COL))?
            .ok_or_else(|| create_error!(UnknownUser))
    }

    /// Find sessions by user id
    async fn fetch_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
        self.col::<Session>(COL)
            .find(doc! {
                "user_id": user_id
            })
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .try_collect()
            .await
            .map_err(|_| create_database_error!("collect", COL))
    }

    /// Find sessions by user ids
    async fn fetch_sessions_with_subscription(&self, user_ids: &[String]) -> Result<Vec<Session>> {
        self.col::<Session>(COL)
            .find(doc! {
                "user_id": {
                    "$in": user_ids
                },
                "subscription": {
                    "$exists": true
                }
            })
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .try_collect()
            .await
            .map_err(|_| create_database_error!("collect", COL))
    }

    /// Fetch a session from the database by token
    async fn fetch_session_by_token(&self, token: &str) -> Result<Session> {
        self.col::<Session>(COL)
            .find_one(doc! {
                "token": token
            })
            .await
            .map_err(|_| create_database_error!("find_one", COL))?
            .ok_or_else(|| create_error!(InvalidSession))
    }

    /// Save session
    async fn save_session(&self, session: &Session) -> Result<()> {
        self.col::<Session>(COL)
            .update_one(
                doc! {
                    "_id": &session.id
                },
                doc! {
                    "$set": to_document(session).map_err(|_| create_database_error!("to_document", COL))?,
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .map_err(|_| create_database_error!("upsert_one", COL))
            .map(|_| ())
    }

    /// Delete session
    async fn delete_session(&self, id: &str) -> Result<()> {
        self.col::<Session>(COL)
            .delete_one(doc! {
                "_id": id
            })
            .await
            .map_err(|_| create_database_error!("delete_one", COL))
            .map(|_| ())
    }

    /// Delete session
    async fn delete_all_sessions(&self, user_id: &str, ignore: Option<String>) -> Result<()> {
        let mut query = doc! {
            "user_id": user_id
        };

        if let Some(id) = ignore {
            query.insert(
                "_id",
                doc! {
                    "$ne": id
                },
            );
        }

        self.col::<Session>(COL)
            .delete_many(query)
            .await
            .map_err(|_| create_database_error!("delete_one", COL))
            .map(|_| ())
    }

    /// Remove push subscription for a session by session id
    async fn remove_push_subscription_by_session_id(&self, session_id: &str) -> Result<()> {
        self.col::<Session>(COL)
            .update_one(
                doc! {
                    "_id": session_id
                },
                doc! {
                    "$unset": {
                        "subscription": 1
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    async fn update_session_last_seen(&self, session_id: &str, when: Timestamp) -> Result<()> {
        self.col::<Session>(COL)
            .update_one(
                doc! {
                    "_id": session_id
                },
                doc! {
                    "$set": {
                        "last_seen": to_bson(&when).unwrap()
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }
}
