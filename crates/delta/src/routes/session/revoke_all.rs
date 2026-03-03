//! Revoke all sessions
//! DELETE /session/all
use revolt_database::{Database, Session, Account};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Delete All Sessions
///
/// Delete all active sessions, optionally including current one.
#[openapi(tag = "Session")]
#[delete("/all?<revoke_self>")]
pub async fn revoke_all(
    db: &State<Database>,
    session: Session,
    account: Account,
    revoke_self: Option<bool>,
) -> Result<EmptyResponse> {
    let ignore = if revoke_self.unwrap_or(false) {
        None
    } else {
        Some(session.id)
    };

    account
        .delete_all_sessions(db, ignore)
        .await
        .map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{Header, Status};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        for i in 1..=3 {
            account
                .create_session(&harness.db, format!("session{}", i))
                .await
                .unwrap();
        }

        let res = harness.client
            .delete("/auth/session/all?revoke_self=true")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
        assert!(harness.db
            .fetch_sessions(&session.user_id)
            .await
            .unwrap()
            .is_empty());
    }

    #[async_std::test]
    async fn success_not_including_self() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        for i in 1..=3 {
            account
                .create_session(&harness.db, format!("session{}", i))
                .await
                .unwrap();
        }

        let res = harness.client
            .delete("/auth/session/all?revoke_self=false")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
        let sessions = harness.db
                .fetch_sessions(&session.user_id)
                .await
                .unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session.id);
    }
}
