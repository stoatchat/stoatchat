//! Fetch available MFA methods.
//! GET /mfa/methods
use revolt_database::Account;
use revolt_models::v0;
use rocket::serde::json::Json;

/// # Get MFA Methods
///
/// Fetch available MFA methods.
#[openapi(tag = "MFA")]
#[get("/methods")]
pub async fn get_mfa_methods(account: Account) -> Json<Vec<v0::MFAMethod>> {
    Json(
        account
            .mfa
            .get_methods()
            .into_iter()
            .map(Into::into)
            .collect(),
    )
}

// #[cfg(test)]
// mod tests {
//     use authifier::models::totp::Totp;

//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, _, _) = for_test_authenticated("get_mfa_methods::success").await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::mfa::get_mfa_methods::get_mfa_methods],
//         )
//         .await;

//         let res = client
//             .get("/methods")
//             .header(Header::new("X-Session-Token", session.token))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert_eq!(
//             serde_json::from_str::<Vec<MFAMethod>>(&res.into_string().await.unwrap()).unwrap(),
//             vec![MFAMethod::Password]
//         );
//     }

//     #[async_std::test]
//     async fn success_has_recovery_and_totp() {
//         use rocket::http::Header;

//         let (authifier, session, mut account, _) =
//             for_test_authenticated("get_mfa_methods::success_has_recovery_and_totp").await;

//         account.mfa.totp_token = Totp::Enabled {
//             secret: "some".to_string(),
//         };
//         account.mfa.generate_recovery_codes();
//         account.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::mfa::get_mfa_methods::get_mfa_methods],
//         )
//         .await;

//         let res = client
//             .get("/methods")
//             .header(Header::new("X-Session-Token", session.token))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert_eq!(
//             serde_json::from_str::<Vec<MFAMethod>>(&res.into_string().await.unwrap()).unwrap(),
//             vec![MFAMethod::Totp, MFAMethod::Recovery]
//         );
//     }
// }
