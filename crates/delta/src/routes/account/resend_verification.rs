//! Resend account verification email
//! POST /account/reverify
use rocket::{serde::json::Json, State};
use rocket_empty::EmptyResponse;
use revolt_result::Result;
use revolt_database::{Database, util::{email::{normalise_email, validate_email}, captcha::check_captcha}, EmailVerification};
use revolt_models::v0;

/// # Resend Verification
///
/// Resend account creation verification email.
#[openapi(tag = "Account")]
#[post("/reverify", data = "<data>")]
pub async fn resend_verification(
    db: &State<Database>,
    data: Json<v0::DataResendVerification>,
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
        match account.verification {
            EmailVerification::Verified => {
                // Send password reset if already verified
                account.start_password_reset(db, true).await?;
            }
            EmailVerification::Pending { .. } => {
                // Resend if not verified yet
                account.start_email_verification(db).await?;
            }
            // Ignore if pending for another email,
            // this should be re-initiated from settings.
            EmailVerification::Moving { .. } => {}
        }
    }

    // Never fail this route,
    // You may open the application to email enumeration otherwise.
    Ok(EmptyResponse)
}

#[cfg(test)]
mod tests {
    use iso8601_timestamp::Timestamp;
    use revolt_database::{Account, EmailVerification};
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{ContentType, Status};
    use revolt_result::{Error, ErrorType};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;

        let mut account = Account::new(
            &harness.db,
            "resend_verification@smtp.test".into(),
            "password".into(),
            false,
        )
        .await
        .unwrap();

        account.verification = EmailVerification::Pending {
            token: "".into(),
            expiry: Timestamp::now_utc(),
        };

        account.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/account/reverify")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "resend_verification@smtp.test",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let (_, code) = harness.assert_email("resend_verification@smtp.test").await;
        let res = harness.client
            .post(format!("/auth/account/verify/{code}"))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
    }

    #[async_std::test]
    async fn success_unknown() {
        let harness = TestHarness::new().await;

        let res = harness.client
            .post("/auth/account/reverify")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "smtptest1@insrt.uk",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }

    #[async_std::test]
    async fn fail_bad_email() {
        let harness = TestHarness::new().await;

        let res = harness.client
            .post("/auth/account/reverify")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "invalid",
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::BadRequest);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::IncorrectData { .. },
        ));
    }
}
