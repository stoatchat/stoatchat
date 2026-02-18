//! Send a password reset email
//! POST /account/reset_password
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_result::Result;
use revolt_database::{Database, EmailVerification, util::{email::{normalise_email, validate_email}, captcha::check_captcha}};
use revolt_models::v0;

/// # Send Password Reset
///
/// Send an email to reset account password.
#[openapi(tag = "Account")]
#[post("/reset_password", data = "<data>")]
pub async fn send_password_reset(
    db: &State<Database>,
    data: Json<v0::DataSendPasswordReset>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    // Check Captcha token
    check_captcha(data.captcha.as_deref()).await?;

    // Make sure email is valid and not blocked
    validate_email(&data.email)?;

    // From this point on, do not report failure to the
    // remote client, as this will open us up to user enumeration.

    // Normalise the email
    let email_normalised = normalise_email(data.email);

    // Try to find the relevant account
    if let Some(mut account) = db
        .fetch_account_by_normalised_email(&email_normalised)
        .await?
    {
        if !matches!(account.verification, EmailVerification::Pending { .. }) {
            account.start_password_reset(db, false).await?;
        }
    }

    // Never fail this route, (except for db error)
    // You may open the application to email enumeration otherwise.
    Ok(EmptyResponse)
}

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         let (authifier, _) =
//             for_test_with_config("send_password_reset::success", test_smtp_config().await).await;

//         Account::new(
//             &authifier,
//             "password_reset@smtp.test".into(),
//             "password".into(),
//             false,
//         )
//         .await
//         .unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![
//                 crate::routes::account::password_reset::password_reset,
//                 crate::routes::account::send_password_reset::send_password_reset,
//                 crate::routes::session::login::login
//             ],
//         )
//         .await;

//         let res = client
//             .post("/reset_password")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "password_reset@smtp.test",
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let mail = assert_email_sendria("password_reset@smtp.test".into()).await;
//         let res = client
//             .patch("/reset_password")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "token": mail.code.expect("`code`"),
//                     "password": "valid password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let res = client
//             .post("/login")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "password_reset@smtp.test",
//                     "password": "valid password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//         assert!(serde_json::from_str::<Session>(&res.into_string().await.unwrap()).is_ok());
//     }
// }
