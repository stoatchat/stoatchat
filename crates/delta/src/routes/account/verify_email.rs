//! Verify an account
//! POST /verify/<code>
use rocket::{serde::json::Json, State};
use revolt_database::{Database, EmailVerification, MFATicket, util::email::normalise_email};
use revolt_result::Result;
use revolt_models::v0;

/// # Verify Email
///
/// Verify an email address.
#[openapi(tag = "Account")]
#[post("/verify/<code>")]
pub async fn verify_email(
    db: &State<Database>,
    code: String,
) -> Result<Json<v0::ResponseVerify>> {
    // Find the account
    let mut account = db
        .fetch_account_with_email_verification(&code)
        .await?;

    // Update account email
    let response = if let EmailVerification::Moving { new_email, .. } = &account.verification {
        account.email = new_email.clone();
        account.email_normalised = normalise_email(new_email.clone());
        v0::ResponseVerify::NoTicket
    } else {
        let mut ticket = MFATicket::new(account.id.to_string(), false);
        ticket.authorised = true;
        ticket.save(db).await?;
        v0::ResponseVerify::WithTicket { ticket: ticket.into() }
    };

    // Mark as verified
    account.verification = EmailVerification::Verified;

    // Save to database
    account.save(db).await?;
    Ok(Json(response))
}

// #[cfg(test)]
// mod tests {
//     use chrono::Duration;
//     use iso8601_timestamp::Timestamp;

//     use crate::test::*;

//     use super::ResponseVerify;

//     #[async_std::test]
//     async fn success() {
//         let (authifier, _, mut account, _) = for_test_authenticated("verify_email::success").await;

//         account.verification = EmailVerification::Pending {
//             token: "token".into(),
//             expiry: Timestamp::from_unix_timestamp_ms(
//                 chrono::Utc::now()
//                     .checked_add_signed(Duration::seconds(100))
//                     .expect("failed to checked_add_signed")
//                     .timestamp_millis(),
//             ),
//         };

//         account.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![
//                 crate::routes::account::verify_email::verify_email,
//                 crate::routes::session::login::login
//             ],
//         )
//         .await;

//         let res = client.post("/verify/token").dispatch().await;

//         assert_eq!(res.status(), Status::Ok);

//         // Make sure it was used and can't be used again
//         assert!(authifier
//             .database
//             .find_account_with_email_verification("token")
//             .await
//             .is_err());

//         // Check that we can login using the received MFA ticket
//         let response =
//             serde_json::from_str::<crate::routes::account::verify_email::ResponseVerify>(
//                 &res.into_string().await.unwrap(),
//             )
//             .expect("`ResponseVerify`");

//         if let ResponseVerify::WithTicket { ticket } = response {
//             let res = client
//                 .post("/login")
//                 .header(ContentType::JSON)
//                 .body(json!({ "mfa_ticket": ticket.token }).to_string())
//                 .dispatch()
//                 .await;

//             assert_eq!(res.status(), Status::Ok);
//             assert!(serde_json::from_str::<Session>(&res.into_string().await.unwrap()).is_ok());
//         } else {
//             panic!("Expected `ResponseVerify::WithTicket`");
//         }
//     }

//     #[async_std::test]
//     async fn fail_invalid_token() {
//         let (authifier, _) = for_test("verify_email::fail_invalid_token").await;

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::verify_email::verify_email],
//         )
//         .await;

//         let res = client.post("/verify/token").dispatch().await;

//         assert_eq!(res.status(), Status::Unauthorized);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"InvalidToken\"}".into())
//         );
//     }
// }
