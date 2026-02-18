//! Fetch recovery codes for an account.
//! POST /mfa/recovery
use rocket::serde::json::Json;
use revolt_database::{Account, ValidatedTicket};
use revolt_result::Result;

/// # Fetch Recovery Codes
///
/// Fetch recovery codes for an account.
#[openapi(tag = "MFA")]
#[post("/recovery")]
pub async fn fetch_recovery(
    account: Account,
    _ticket: ValidatedTicket,
) -> Result<Json<Vec<String>>> {
    Ok(Json(account.mfa.recovery_codes))
}

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, account, _) =
//             for_test_authenticated("fetch_recovery::success").await;
//         let ticket = MFATicket::new(account.id, true);
//         ticket.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::mfa::fetch_recovery::fetch_recovery],
//         )
//         .await;

//         let res = client
//             .post("/recovery")
//             .header(Header::new("X-Session-Token", session.token))
//             .header(Header::new("X-MFA-Ticket", ticket.token))
//             .header(ContentType::JSON)
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert!(
//             serde_json::from_str::<Vec<String>>(&res.into_string().await.unwrap())
//                 .unwrap()
//                 .is_empty()
//         );
//     }
// }
