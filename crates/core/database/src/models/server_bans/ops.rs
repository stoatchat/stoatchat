use revolt_result::Result;

use crate::{MemberCompositeKey, ServerBan};

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractServerBans: Sync + Send {
    /// Insert new ban into database
    async fn insert_ban(&self, ban: &ServerBan) -> Result<()>;

    /// Fetch a server ban by server and user id
    async fn fetch_ban(&self, server_id: &str, user_id: &str) -> Result<ServerBan>;

    /// Fetch all bans in a server
    async fn fetch_bans(&self, server_id: &str) -> Result<Vec<ServerBan>>;

    /// Delete a ban from the database
    async fn delete_ban(&self, id: &MemberCompositeKey) -> Result<()>;
}
