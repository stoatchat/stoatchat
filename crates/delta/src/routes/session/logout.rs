//! Logout of current session
//! POST /session/logout
use revolt_database::{Database, Session};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Logout
///
/// Delete current session.
#[openapi(tag = "Session")]
#[post("/logout")]
pub async fn logout(db: &State<Database>, session: Session) -> Result<EmptyResponse> {
    session.delete(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::events::client::EventV1;
    use revolt_result::ErrorType;
    use rocket::http::{Header, Status};

    #[async_std::test]
    async fn success() {
        let mut harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness.client
            .post("/auth/session/logout")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
        drop(res);
        assert!(matches!(
            harness.db
                .fetch_session(&session.id)
                .await
                .unwrap_err().error_type,
            ErrorType::UnknownUser
        ));

        let event = harness.wait_for_event(&format!("{}!", &session.user_id), |evt| matches!(evt, EventV1::DeleteSession { .. })).await;
        if let EventV1::DeleteSession {
            user_id,
            session_id,
        } = event
        {
            assert_eq!(user_id, session.user_id);
            assert_eq!(session_id, session.id);
        } else {
            panic!("Received incorrect event type. {:?}", event);
        }
    }

    #[async_std::test]
    async fn fail_invalid_session() {
        let harness = TestHarness::new().await;

        let res = harness.client
            .post("/auth/session/logout")
            .header(Header::new("X-Session-Token", "invalid"))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[async_std::test]
    async fn fail_no_session() {
        let harness = TestHarness::new().await;

        let res = harness.client.post("/auth/session/logout").dispatch().await;

        assert_eq!(res.status(), Status::Unauthorized);
    }
}
