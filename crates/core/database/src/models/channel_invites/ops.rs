use revolt_result::Result;

use crate::Invite;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractChannelInvites: Sync + Send {
    /// Insert a new invite into the database
    async fn insert_invite(&self, invite: &Invite) -> Result<()>;

    /// Fetch an invite by its id
    async fn fetch_invite(&self, code: &str) -> Result<Invite>;

    /// Fetch all invites for a server
    async fn fetch_invites_for_server(&self, server_id: &str) -> Result<Vec<Invite>>;

    /// Delete an invite by its id
    async fn delete_invite(&self, code: &str) -> Result<()>;
}
