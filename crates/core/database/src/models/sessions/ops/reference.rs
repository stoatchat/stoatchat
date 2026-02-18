use crate::{AbstractSessions, ReferenceDb, Session};
use iso8601_timestamp::Timestamp;
use revolt_result::Result;

#[async_trait]
impl AbstractSessions for ReferenceDb {
    /// Find session by id
    async fn fetch_session(&self, id: &str) -> Result<Session> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(UnknownUser))
    }

    /// Find sessions by user id
    async fn fetch_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
        let sessions = self.sessions.lock().await;
        Ok(sessions
            .values()
            .filter(|session| session.user_id == user_id)
            .cloned()
            .collect())
    }

    /// Find sessions by user ids
    async fn fetch_sessions_with_subscription(&self, user_ids: &[String]) -> Result<Vec<Session>> {
        let sessions = self.sessions.lock().await;
        Ok(sessions
            .values()
            .filter(|session| session.subscription.is_some() && user_ids.contains(&session.id))
            .cloned()
            .collect())
    }

    /// Find session by token
    async fn fetch_session_by_token(&self, token: &str) -> Result<Session> {
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .find(|session| session.token == token)
            .cloned()
            .ok_or_else(|| create_error!(InvalidSession))
    }

    /// Save session
    async fn save_session(&self, session: &Session) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session.id.to_string(), session.clone());
        Ok(())
    }

    /// Delete session
    async fn delete_session(&self, id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        if sessions.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(InvalidSession))
        }
    }

    /// Delete session
    async fn delete_all_sessions(&self, user_id: &str, ignore: Option<String>) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|_, session| {
            if session.user_id == user_id {
                if let Some(ignore) = &ignore {
                    ignore == &session.id
                } else {
                    false
                }
            } else {
                true
            }
        });

        Ok(())
    }

    /// Remove push subscription for a session by session id
    async fn remove_push_subscription_by_session_id(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(session_id) {
            session.subscription = None;
        };

        Ok(())
    }

    async fn update_session_last_seen(&self, session_id: &str, when: Timestamp) -> Result<()> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(session_id) {
            session.last_seen = when;
        };

        Ok(())
    }
}
