//! Edit a session
//! PATCH /session/:id
use revolt_database::{Database, Session};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::serde::json::Json;
use rocket::State;


/// # Edit Session
///
/// Edit current session information.
#[openapi(tag = "Session")]
#[patch("/<id>", data = "<data>")]
pub async fn edit(
    db: &State<Database>,
    user: Session,
    id: String,
    data: Json<v0::DataEditSession>,
) -> Result<Json<v0::SessionInfo>> {
    let mut session = db.fetch_session(&id).await?;

    // Make sure we own this session
    if user.user_id != session.user_id {
        return Err(create_error!(InvalidSession));
    }

    // Rename the session
    session.name = data.into_inner().friendly_name;

    // Save session
    session.save(db).await?;

    Ok(Json(session.into()))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{ContentType, Status};
    use revolt_models::v0;

    #[async_std::test]
    async fn success() {
        use rocket::http::Header;

        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness.client
            .patch(format!("/auth/session/{}", session.id))
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token))
            .body(
                json!({
                    "friendly_name": "test name"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);

        assert_eq!(res.into_json::<v0::SessionInfo>().await.unwrap().name, "test name");
    }
}
