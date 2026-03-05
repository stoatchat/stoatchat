//! Fetch all sessions
//! GET /session/all
use revolt_result::Result;
use revolt_database::{Database, Session};
use revolt_models::v0;
use rocket::serde::json::Json;
use rocket::State;

/// # Fetch Sessions
///
/// Fetch all sessions associated with this account.
#[openapi(tag = "Session")]
#[get("/all")]
pub async fn fetch_all(
    db: &State<Database>,
    session: Session,
) -> Result<Json<Vec<v0::SessionInfo>>> {
    db
        .fetch_sessions(&session.user_id)
        .await
        .map(|ok| ok.into_iter().map(|session| session.into()).collect())
        .map(Json)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{Header, Status};
    use revolt_models::v0;

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
            .get("/auth/session/all")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Vec<v0::SessionInfo>>().await.unwrap().len(), 4);
    }
}
