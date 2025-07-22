#[cfg(feature = "mongodb")]
use ::mongodb::{ClientSession, SessionCursor};

use revolt_result::Result;

use crate::{FieldsMember, Member, MemberCompositeKey, PartialMember};

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ChunkedServerMembersGenerator {
    #[cfg(feature = "mongodb")]
    MongoDb {
        session: ClientSession,
        cursor: Option<SessionCursor<Member>>,
    },

    Reference {
        offset: i32,
        data: Option<Vec<Member>>,
    },
}

impl ChunkedServerMembersGenerator {
    #[cfg(feature = "mongodb")]
    pub fn new_mongo(session: ClientSession, cursor: SessionCursor<Member>) -> Self {
        ChunkedServerMembersGenerator::MongoDb {
            session,
            cursor: Some(cursor),
        }
    }

    pub fn new_reference(data: Vec<Member>) -> Self {
        ChunkedServerMembersGenerator::Reference {
            offset: 0,
            data: Some(data),
        }
    }

    pub async fn next(&mut self) -> Option<Member> {
        match self {
            #[cfg(feature = "mongodb")]
            ChunkedServerMembersGenerator::MongoDb { session, cursor } => {
                if let Some(cursor) = cursor {
                    let value = cursor.next(session).await;
                    value.map(|val| val.expect("Failed to fetch the next member"))
                } else {
                    warn!("Attempted to access a (MongoDb) server member generator without first setting a cursor");
                    None
                }
            }
            ChunkedServerMembersGenerator::Reference { offset, data } => {
                if let Some(data) = data {
                    if data.len() as i32 >= *offset {
                        None
                    } else {
                        let resp = &data[*offset as usize];
                        *offset += 1;
                        Some(resp.clone())
                    }
                } else {
                    warn!("Attempted to access a (Reference) server member generator without first providing data");
                    None
                }
            }
        }
    }
}

#[async_trait]
pub trait AbstractServerMembers: Sync + Send {
    /// Insert a new server member into the database
    async fn insert_or_merge_member(&self, member: &Member) -> Result<Option<Member>>;

    /// Fetch a server member by their id
    async fn fetch_member(&self, server_id: &str, user_id: &str) -> Result<Member>;

    /// Fetch all members in a server
    async fn fetch_all_members(&self, server_id: &str) -> Result<Vec<Member>>;

    /// Fetch all members in a server as an iterator
    async fn fetch_all_members_chunked(
        &self,
        server_id: &str,
    ) -> Result<ChunkedServerMembersGenerator>;

    async fn fetch_all_members_with_roles(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<Vec<Member>>;

    async fn fetch_all_members_with_roles_chunked(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<ChunkedServerMembersGenerator>;

    /// Fetch all memberships for a user
    async fn fetch_all_memberships(&self, user_id: &str) -> Result<Vec<Member>>;

    /// Fetch multiple members by their ids
    async fn fetch_members(&self, server_id: &str, ids: &[String]) -> Result<Vec<Member>>;

    /// Fetch member count of a server
    async fn fetch_member_count(&self, server_id: &str) -> Result<usize>;

    /// Fetch server count of a user
    async fn fetch_server_count(&self, user_id: &str) -> Result<usize>;

    /// Update information for a server member
    async fn update_member(
        &self,
        id: &MemberCompositeKey,
        partial: &PartialMember,
        remove: Vec<FieldsMember>,
    ) -> Result<()>;

    /// Marks a user as no longer a member of a server, while retaining the database value.
    /// This is used to keep information such as timeouts in place, but will remove information such as join date and applied roles.
    async fn soft_delete_member(&self, id: &MemberCompositeKey) -> Result<()>;

    /// Forcibly delete a server member by their id.
    /// This will cancel any pending timeouts or other longer term actions, and they will not be reapplied on rejoin.
    async fn force_delete_member(&self, id: &MemberCompositeKey) -> Result<()>;

    /// Fetch all members who have been marked for deletion.
    async fn remove_dangling_members(&self) -> Result<()>;

    /// Fetch all participants of a server.
    async fn fetch_server_participants(
        &self,
        server_id: &str,
    ) -> Result<Vec<(crate::User, Option<Member>)>>;

    /// Fetch all members of a server
    async fn fetch_server_members(
        &self,
        server_id: &str,
        page_size: u8,
        after: Option<usize>,
    ) -> Result<Vec<(crate::User, Member)>>;
}
