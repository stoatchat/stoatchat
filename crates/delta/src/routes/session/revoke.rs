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

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, _, _) = for_test_authenticated("revoke::success").await;
//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![crate::routes::session::revoke::revoke],
//         )
//         .await;

//         let res = client
//             .delete(format!("/{}", session.id))
//             .header(Header::new("X-Session-Token", session.token))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);
//         assert_eq!(
//             authifier
//                 .database
//                 .find_session(&session.id)
//                 .await
//                 .unwrap_err(),
//             Error::UnknownUser
//         );
//     }
// }
