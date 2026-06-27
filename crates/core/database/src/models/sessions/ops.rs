use iso8601_timestamp::Timestamp;
use revolt_result::Result;

use crate::Session;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractSessions: Sync + Send {
    /// Find session by id
    async fn fetch_session(&self, id: &str) -> Result<Session>;

    /// Find sessions by user id
    async fn fetch_sessions(&self, user_id: &str) -> Result<Vec<Session>>;

    /// Find sessions by user ids
    async fn fetch_sessions_with_subscription(&self, user_ids: &[String]) -> Result<Vec<Session>>;

    /// Find session by token
    async fn fetch_session_by_token(&self, token: &str) -> Result<Session>;

    /// Save session
    async fn save_session(&self, session: &Session) -> Result<()>;

    /// Delete session
    async fn delete_session(&self, id: &str) -> Result<()>;

    /// Delete session
    async fn delete_all_sessions(&self, user_id: &str, ignore: Option<String>) -> Result<()>;

    /// Remove push subscription for a session by session id
    async fn remove_push_subscription_by_session_id(&self, session_id: &str) -> Result<()>;

    async fn update_session_last_seen(&self, session_id: &str, when: Timestamp) -> Result<()>;
}
