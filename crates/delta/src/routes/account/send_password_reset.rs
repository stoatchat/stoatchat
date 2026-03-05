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

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::Account;
    use revolt_models::v0;
    use rocket::http::{ContentType, Status};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;

        Account::new(
            &harness.db,
            "password_reset@smtp.test".into(),
            "password".into(),
            false,
        )
        .await
        .unwrap();

        let res = harness.client
            .post("/auth/account/reset_password")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "password_reset@smtp.test",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let (_, code) = harness.assert_email("password_reset@smtp.test").await;
        let res = harness.client
            .patch("/auth/account/reset_password")
            .header(ContentType::JSON)
            .body(
                json!({
                    "token": code,
                    "password": "valid password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let res = harness.client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "password_reset@smtp.test",
                    "password": "valid password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(serde_json::from_str::<v0::Session>(&res.into_string().await.unwrap()).is_ok());
    }
}
