//! Fetch MFA status of an account.
//! GET /mfa
use revolt_database::Account;
use revolt_result::Result;
use revolt_models::v0;
use rocket::serde::json::Json;

/// # MFA Status
///
/// Fetch MFA status of an account.
#[openapi(tag = "MFA")]
#[get("/")]
pub async fn fetch_status(account: Account) -> Result<Json<v0::MultiFactorStatus>> {
    Ok(Json(account.mfa.into()))
}

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, _, _) = for_test_authenticated("fetch_status::success").await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::mfa::fetch_status::fetch_status],
//         )
//         .await;

//         let res = client
//             .get("/")
//             .header(Header::new("X-Session-Token", session.token))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert!(
//             serde_json::from_str::<crate::routes::mfa::fetch_status::MultiFactorStatus>(
//                 &res.into_string().await.unwrap()
//             )
//             .is_ok()
//         );
//     }
// }
