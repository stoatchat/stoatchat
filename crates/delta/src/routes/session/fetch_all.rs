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

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, account, _) = for_test_authenticated("fetch_all::success").await;

//         for i in 1..=3 {
//             account
//                 .create_session(&authifier, format!("session{}", i))
//                 .await
//                 .unwrap();
//         }

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::session::fetch_all::fetch_all],
//         )
//         .await;

//         let res = client
//             .get("/all")
//             .header(Header::new("X-Session-Token", session.token))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);

//         let result = res.into_string().await.unwrap();
//         let sessions: Vec<crate::routes::session::fetch_all::SessionInfo> =
//             serde_json::from_str(&result).unwrap();
//         assert_eq!(sessions.len(), 4);
//     }
// }
