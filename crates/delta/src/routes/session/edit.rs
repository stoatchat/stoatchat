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

// #[cfg(test)]
// mod tests {
//     use crate::{routes::session::fetch_all::SessionInfo, test::*};

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, _, _) = for_test_authenticated("edit::success").await;
//         let client =
//             bootstrap_rocket_with_auth(authifier, routes![crate::routes::session::edit::edit])
//                 .await;

//         let res = client
//             .patch(format!("/{}", session.id))
//             .header(ContentType::JSON)
//             .header(Header::new("X-Session-Token", session.token))
//             .body(
//                 json!({
//                     "friendly_name": "test name"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);

//         let result = res.into_string().await.unwrap();
//         let session = serde_json::from_str::<SessionInfo>(&result).unwrap();
//         assert_eq!(session.name, "test name");
//     }
// }
