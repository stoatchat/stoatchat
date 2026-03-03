//! Revoke an active session
//! DELETE /session/:id
use revolt_database::{Database, Session};
use revolt_result::{Result, create_error};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Revoke Session
///
/// Delete a specific active session.
#[openapi(tag = "Session")]
#[delete("/<id>")]
pub async fn revoke(
    db: &State<Database>,
    user: Session,
    id: String,
) -> Result<EmptyResponse> {
    let session = db.fetch_session(&id).await?;

    if session.user_id != user.user_id {
        return Err(create_error!(InvalidToken));
    }

    session.delete(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_result::ErrorType;
    use rocket::http::{ContentType, Header, Status};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness.client
            .delete(format!("/auth/session/{}", session.id))
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
        assert!(matches!(
            harness.db
                .fetch_session(&session.id)
                .await
                .unwrap_err()
                .error_type,
            ErrorType::UnknownUser
        ));
    }
}
