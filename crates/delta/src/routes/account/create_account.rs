//! Create a new account
//! POST /account/create
use revolt_config::config;
use revolt_database::{
    Account, Database, util::{captcha::check_captcha, email::validate_email, password::assert_safe, shield::{ShieldValidationInput, validate_shield}}
};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Create Account
///
/// Create a new account.
#[openapi(tag = "Account")]
#[post("/create", data = "<data>")]
pub async fn create_account(
    db: &State<Database>,
    data: Json<v0::DataCreateAccount>,
    mut shield: ShieldValidationInput,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    // Check Captcha token
    check_captcha(data.captcha.as_deref()).await?;

    // Validate the request
    shield.email = Some(data.email.to_string());
    validate_shield(shield).await?;

    // Make sure email is valid and not blocked
    validate_email(&data.email)?;

    // Ensure password is safe to use
    assert_safe(&data.password)
        .await?;

    // If required, fetch valid invite
    let invite = if config().await.api.registration.invite_only {
        if let Some(invite) = data.invite {
            Some(db.fetch_account_invite(&invite).await?)
        } else {
            return Err(create_error!(MissingInvite));
        }
    } else {
        None
    };

    // Create account
    let account = Account::new(db, data.email, data.password, true).await?;

    // Use up the invite
    if let Some(mut invite) = invite {
        invite.claimed_by = Some(account.id);
        invite.used = true;

        db.save_account_invite(&invite).await?;
    }

    Ok(EmptyResponse)
}

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         let (client, receiver) = bootstrap_rocket(
//             "create_account",
//             "success",
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "valid password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let event = receiver.try_recv().expect("an event");
//         if !matches!(event, AuthifierEvent::CreateAccount { .. }) {
//             panic!("Received incorrect event type. {:?}", event);
//         }
//     }

//     #[async_std::test]
//     async fn fail_invalid_email() {
//         let (client, _) = bootstrap_rocket(
//             "create_account",
//             "fail_invalid_email",
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "invalid",
//                     "password": "valid password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::BadRequest);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"IncorrectData\",\"with\":\"email\"}".into())
//         );
//     }

//     #[async_std::test]
//     async fn fail_invalid_password() {
//         let (client, _) = bootstrap_rocket(
//             "create_account",
//             "fail_invalid_password",
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "password"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::BadRequest);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"CompromisedPassword\"}".into())
//         );
//     }

//     #[async_std::test]
//     async fn fail_invalid_invite() {
//         let config = Config {
//             invite_only: true,
//             ..Default::default()
//         };

//         let (authifier, _) =
//             for_test_with_config("create_account::fail_invalid_invite", config).await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "valid password",
//                     "invite": "invalid"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::BadRequest);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"InvalidInvite\"}".into())
//         );
//     }

//     #[async_std::test]
//     async fn success_valid_invite() {
//         let config = Config {
//             invite_only: true,
//             ..Default::default()
//         };

//         let (authifier, _) =
//             for_test_with_config("create_account::success_valid_invite", config).await;
//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let invite = Invite {
//             id: "invite".to_string(),
//             used: false,
//             claimed_by: None,
//         };

//         authifier.database.save_invite(&invite).await.unwrap();

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "valid password",
//                     "invite": "invite"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let invite = authifier
//             .database
//             .find_invite("invite")
//             .await
//             .expect("`Invite`");

//         assert!(invite.used);
//     }

//     #[async_std::test]
//     async fn fail_missing_captcha() {
//         let config = Config {
//             captcha: Captcha::HCaptcha {
//                 secret: "0x0000000000000000000000000000000000000000".into(),
//             },
//             ..Default::default()
//         };

//         let (authifier, _) =
//             for_test_with_config("create_account::fail_missing_captcha", config).await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "valid password",
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::BadRequest);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"CaptchaFailed\"}".into())
//         );
//     }

//     #[async_std::test]
//     async fn fail_captcha_invalid() {
//         let config = Config {
//             captcha: Captcha::HCaptcha {
//                 secret: "0x0000000000000000000000000000000000000000".into(),
//             },
//             ..Default::default()
//         };

//         let (authifier, _) =
//             for_test_with_config("create_account::fail_invalid_captcha", config).await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "valid password",
//                     "captcha": "00000000-aaaa-bbbb-cccc-000000000000"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::BadRequest);
//         assert_eq!(
//             res.into_string().await,
//             Some("{\"type\":\"CaptchaFailed\"}".into())
//         );
//     }

//     #[async_std::test]
//     async fn success_captcha_valid() {
//         let config = Config {
//             captcha: Captcha::HCaptcha {
//                 secret: "0x0000000000000000000000000000000000000000".into(),
//             },
//             ..Default::default()
//         };

//         let (authifier, _) = for_test_with_config("create_account::success_captcha", config).await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::create_account::create_account],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "example@validemail.com",
//                     "password": "valid password",
//                     "captcha": "20000000-aaaa-bbbb-cccc-000000000002"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);
//     }

//     #[async_std::test]
//     async fn success_smtp_sent() {
//         let (authifier, _) = for_test_with_config(
//             "create_account::success_smtp_sent",
//             test_smtp_config().await,
//         )
//         .await;
//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![
//                 crate::routes::account::create_account::create_account,
//                 crate::routes::account::verify_email::verify_email
//             ],
//         )
//         .await;

//         let res = client
//             .post("/create")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "email": "create_account@smtp.test",
//                     "password": "valid password",
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let mail = assert_email_sendria("create_account@smtp.test".into()).await;
//         let res = client
//             .post(format!("/verify/{}", mail.code.expect("`code`")))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);
//     }
// }
