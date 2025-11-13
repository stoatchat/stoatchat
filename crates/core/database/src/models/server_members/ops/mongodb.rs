use bson::Document;
use futures::StreamExt;
use iso8601_timestamp::Timestamp;
use mongodb::options::ReadConcern;
use revolt_result::Result;

use crate::{FieldsMember, Member, MemberCompositeKey, PartialMember};
use crate::{IntoDocumentPath, MongoDb};

use super::{AbstractServerMembers, ChunkedServerMembersGenerator};

static COL: &str = "server_members";

#[async_trait]
impl AbstractServerMembers for MongoDb {
    /// Insert a new server member (or use the existing member if one is found)
    async fn insert_or_merge_member(&self, member: &Member) -> Result<Option<Member>> {
        let existing: Result<Option<Document>> = query!(
            self,
            find_one,
            COL,
            doc! {
                "_id.server": &member.id.server,
                "_id.user": &member.id.user,
                "pending_deletion_at": {"$exists": true}
            }
        );
        // Update the existing record if it exist, otherwise make a new record
        if existing.is_ok_and(|x| x.is_some()) {
            self.col::<Member>(COL)
                .find_one_and_update(
                    doc! {
                        "_id.server": &member.id.server,
                        "_id.user": &member.id.user,
                    },
                    doc! {
                        "$set": {
                            "joined_at": member.joined_at.duration_since(Timestamp::UNIX_EPOCH).whole_seconds(),
                        },
                        "$unset": {
                            "pending_deletion_at": ""
                        }
                    },
                )
                .return_document(mongodb::options::ReturnDocument::After)
                .await
                .map_err(|_| create_database_error!("update_one", COL))
        } else {
            query!(self, insert_one, COL, &member).map(|_| ())?;
            Ok(None)
        }
    }

    /// Fetch a server member by their id
    async fn fetch_member(&self, server_id: &str, user_id: &str) -> Result<Member> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "_id.server": server_id,
                "_id.user": user_id,
                "pending_deletion_at": {"$exists": false}
            }
        )?
        .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all members in a server
    async fn fetch_all_members(&self, server_id: &str) -> Result<Vec<Member>> {
        Ok(self
            .col::<Member>(COL)
            .find(doc! {
                "_id.server": server_id,
                "pending_deletion_at": {"$exists": false}
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

    /// Fetch all members in a server as a generator.
    /// Uses config key pushd.mass_mention_chunk_size as the batch size.
    async fn fetch_all_members_chunked(
        &self,
        server_id: &str,
    ) -> Result<ChunkedServerMembersGenerator> {
        let config = revolt_config::config().await;

        let mut session = self
            .start_session()
            .await
            .map_err(|_| create_database_error!("start_session", COL))?;

        session
            .start_transaction()
            .read_concern(ReadConcern::snapshot())
            .await
            .map_err(|_| create_database_error!("start_transaction", COL))?;

        let cursor = self
            .col::<Member>(COL)
            .find(doc! {
                "_id.server": server_id
            })
            .session(&mut session)
            .batch_size(config.pushd.mass_mention_chunk_size as u32)
            .await
            .map_err(|_| create_database_error!("find", COL))?;

        Ok(ChunkedServerMembersGenerator::new_mongo(session, cursor))
    }

    async fn fetch_all_members_with_roles(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<Vec<Member>> {
        Ok(self
            .col::<Member>(COL)
            .find(doc! {
                "_id.server": server_id,
                "roles": {"$in": roles}
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

    async fn fetch_all_members_with_roles_chunked(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<ChunkedServerMembersGenerator> {
        let config = revolt_config::config().await;

        let mut session = self
            .start_session()
            .await
            .map_err(|_| create_database_error!("start_session", COL))?;

        session
            .start_transaction()
            .read_concern(ReadConcern::snapshot())
            .await
            .map_err(|_| create_database_error!("start_transaction", COL))?;

        let cursor = self
            .col::<Member>(COL)
            .find(doc! {
                "_id.server": server_id,
                "roles": {"$in": roles}
            })
            .session(&mut session)
            .batch_size(config.pushd.mass_mention_chunk_size as u32)
            .await
            .map_err(|_| create_database_error!("find", COL))?;

        return Ok(ChunkedServerMembersGenerator::new_mongo(session, cursor));
    }

    /// Fetch all memberships for a user
    async fn fetch_all_memberships(&self, user_id: &str) -> Result<Vec<Member>> {
        Ok(self
            .col::<Member>(COL)
            .find(doc! {
                "_id.user": user_id,
                "pending_deletion_at": {"$exists": false}
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

    /// Fetch multiple members by their ids
    async fn fetch_members(&self, server_id: &str, ids: &[String]) -> Result<Vec<Member>> {
        Ok(self
            .col::<Member>(COL)
            .find(doc! {
                "_id.server": server_id,
                "pending_deletion_at": {"$exists": false},
                "_id.user": {
                    "$in": ids
                }
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

    /// Fetch member count of a server
    async fn fetch_member_count(&self, server_id: &str) -> Result<usize> {
        self.col::<Member>(COL)
            .count_documents(doc! {
                "_id.server": server_id,
                "pending_deletion_at": {"$exists": false}
            })
            .await
            .map(|c| c as usize)
            .map_err(|_| create_database_error!("count_documents", COL))
    }

    /// Fetch server count of a user
    async fn fetch_server_count(&self, user_id: &str) -> Result<usize> {
        self.col::<Member>(COL)
            .count_documents(doc! {
                "_id.user": user_id,
                "pending_deletion_at": {"$exists": false}
            })
            .await
            .map(|c| c as usize)
            .map_err(|_| create_database_error!("count_documents", COL))
    }

    /// Update information for a server member
    async fn update_member(
        &self,
        id: &MemberCompositeKey,
        partial: &PartialMember,
        remove: Vec<FieldsMember>,
    ) -> Result<()> {
        query!(
            self,
            update_one,
            COL,
            doc! {
                "_id.server": &id.server,
                "_id.user": &id.user
            },
            partial,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            None
        )
        .map(|_| ())
    }

    /// Marks a member for deletion.
    /// This will remove the record if the user has no pending actions (eg. timeout),
    /// otherwise will slate the record for deletion by revolt_crond once the actions expire.
    async fn soft_delete_member(&self, id: &MemberCompositeKey) -> Result<()> {
        let member = self.fetch_member(&id.server, &id.user).await;
        if let Ok(member) = member {
            if member.in_timeout() {
                self.col::<Document>(COL)
                    .update_many(
                        doc! {
                            "_id.server": &id.server,
                            "_id.user": &id.user,
                        },
                        doc! {
                            "$set": {"pending_deletion_at": format!("{}", member.timeout.unwrap().format())},
                            "$unset": {
                                "joined_at": "",
                                "avatar": "",
                                "nickname": "",
                                "roles": ""
                            }
                        },
                    )
                    .await
                    .map(|_| ())
                    .map_err(|_| create_database_error!("update_many", COL))
            } else {
                self.force_delete_member(id).await
            }
        } else {
            Err(create_database_error!("fetch_member", COL))
        }
    }

    /// Delete a server member by their id
    async fn force_delete_member(&self, id: &MemberCompositeKey) -> Result<()> {
        query!(
            self,
            delete_one,
            COL,
            doc! {
                "_id.server": &id.server,
                "_id.user": &id.user
            }
        )
        .map(|_| ())
    }

    async fn remove_dangling_members(&self) -> Result<()> {
        let now = Timestamp::now_utc();
        let date = bson::to_bson(&now).expect("Failed to serialize timestamp");

        self.col::<Document>(COL)
            .delete_many(doc! {
                "pending_deletion_at": {"$lt": date}
            })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("count_documents", COL))
    }
}

impl IntoDocumentPath for FieldsMember {
    fn as_path(&self) -> Option<&'static str> {
        Some(match self {
            FieldsMember::JoinedAt => "joined_at",
            FieldsMember::Avatar => "avatar",
            FieldsMember::Nickname => "nickname",
            FieldsMember::Roles => "roles",
            FieldsMember::Timeout => "timeout",
            FieldsMember::CanPublish => "can_publish",
            FieldsMember::CanReceive => "can_receive",
        })
    }
}
