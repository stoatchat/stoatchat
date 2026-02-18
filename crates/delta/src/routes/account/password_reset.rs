//! Confirm a password reset.
//! PATCH /account/reset_password
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_database::util::password::{hash_password, assert_safe};
use revolt_database::{Database};
use revolt_models::v0;
use revolt_result::Result;

/// # Password Reset
///
/// Confirm password reset and change the password.
#[openapi(tag = "Account")]
#[patch("/reset_password", data = "<data>")]
pub async fn password_reset(
    db: &State<Database>,
    data: Json<v0::DataPasswordReset>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    // Find the relevant account
    let mut account = db
        .fetch_account_with_password_reset(&data.token)
        .await?;

    // Verify password can be used
    assert_safe(&data.password)
        .await?;

    // Update the account
    account.password = hash_password(data.password)?;
    account.password_reset = None;
    account.lockout = None;

    // Commit to database
    account.save(db).await?;

    // Delete all sessions if required
    if data.remove_sessions {
        account.delete_all_sessions(db, None).await?;
    }

    Ok(EmptyResponse)
}

// #[cfg(test)]
// mod tests {
//     use chrono::Duration;
//     use iso8601_timestamp::Timestamp;

//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         let (authifier, session, mut account, _) =
//             for_test_authenticated("password_reset::success").await;

//         account.password_reset = Some(PasswordReset {
//             token: "token".into(),
//             expiry: Timestamp::from_unix_timestamp_ms(
//                 chrono::Utc::now()
//                     .checked_add_signed(Duration::seconds(100))
//                     .expect("failed to checked_add_signed")
//                     .timestamp_millis(),
//             ),
//         });

//         account.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![
//                 crate::routes::account::password_reset::password_reset,
//                 crate::routes::session::login::login
//             ],
//         )
//         .await;

//         let res = client
//             .patch("/reset_password")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "token": "token",
//                     "password": "valid password",
//                     "remove_sessions": true
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         // Make sure it was used and can't be used again
//         assert!(authifier
//             .database
//             .find_account_with_password_reset("token")
//             .await
//             .is_err());

//         let res = client
//             .post("/login")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "email@revolt.chat",
//                     "password": "valid password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert!(serde_json::from_str::<Session>(&res.into_string().await.unwrap()).is_ok());

//         // Ensure sessions were deleted
//         assert_eq!(
//             authifier
//                 .database
//                 .find_session(&session.id)
//                 .await
//                 .unwrap_err(),
//             Error::UnknownUser
//         );
//     }

//     #[async_std::test]
//     async fn fail_invalid_token() {
//         let (authifier, _) = for_test("password_reset::fail_invalid_token").await;

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::password_reset::password_reset],
//         )
//         .await;

//         let res = client
//             .patch("/reset_password")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "token": "invalid",
//                     "password": "valid password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Unauthorized);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"InvalidToken\"}".into())
//         );
//     }
// }
