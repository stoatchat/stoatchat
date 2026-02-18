//! Fetch your account
//! GET /account
use revolt_database::Account;
use rocket::serde::json::Json;
use revolt_models::v0;
use revolt_result::Result;

/// # Fetch Account
///
/// Fetch account information from the current session.
#[openapi(tag = "Account")]
#[get("/")]
pub async fn fetch_account(account: Account) -> Result<Json<v0::AccountInfo>> {
    Ok(Json(account.into()))
}

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, _, _) = for_test_authenticated("fetch_account::success").await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::fetch_account::fetch_account],
//         )
//         .await;

//         let res = client
//             .get("/")
//             .header(Header::new("X-Session-Token", session.token))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert!(
//             serde_json::from_str::<crate::routes::account::fetch_account::AccountInfo>(
//                 &res.into_string().await.unwrap()
//             )
//             .is_ok()
//         );
//     }
// }
