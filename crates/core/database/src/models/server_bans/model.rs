use revolt_result::Result;

use crate::{Database, MemberCompositeKey, Server};

auto_derived!(
    /// Server Ban
    pub struct ServerBan {
        /// Unique member id
        #[serde(rename = "_id")]
        pub id: MemberCompositeKey,
        /// Reason for ban creation
        pub reason: Option<String>,
    }
);

#[allow(clippy::disallowed_methods)]
impl ServerBan {
    /// Create ban
    pub async fn create(
        db: &Database,
        server: &Server,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<ServerBan> {
        let ban = ServerBan {
            id: MemberCompositeKey {
                server: server.id.to_string(),
                user: user_id.to_string(),
            },
            reason,
        };

        db.insert_ban(&ban).await?;
        Ok(ban)
    }
}
