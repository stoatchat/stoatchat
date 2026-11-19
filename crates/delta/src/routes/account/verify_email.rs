//! Verify an account
//! POST /verify/<code>
use revolt_database::{AMQP, Database, EmailVerification, MFATicket, util::email::normalise_email};
use revolt_models::v0;
use revolt_result::Result;
use rocket::{State, serde::json::Json};

/// # Verify Email
///
/// Verify an email address.
#[openapi(tag = "Account")]
#[post("/verify/<code>")]
pub async fn verify_email(
    db: &State<Database>,
    amqp: &State<AMQP>,
    code: String,
) -> Result<Json<v0::ResponseVerify>> {
    // Find the account
    let mut account = db.fetch_account_with_email_verification(&code).await?;

    let old_email = account.email.clone();
    let is_moving: bool;

    // Update account email
    let response = if let EmailVerification::Moving { new_email, .. } = &account.verification {
        account.email = new_email.clone();
        account.email_normalised = normalise_email(new_email.clone());
        is_moving = true;
        v0::ResponseVerify::NoTicket
    } else {
        let mut ticket = MFATicket::new(account.id.to_string(), false);
        ticket.authorised = true;
        ticket.save(db).await?;
        is_moving = false;
        v0::ResponseVerify::WithTicket {
            ticket: ticket.into(),
        }
    };

    // Mark as verified
    account.verification = EmailVerification::Verified;

    // Save to database
    account.save(db).await?;

    if is_moving && let Ok(user) = db.fetch_user(&account.id).await {
        _ = user
            .send_system_notice(
                db,
                amqp,
                revolt_database::SystemNotice::EmailChanged {
                    from: old_email,
                    to: account.email.clone(),
                },
            )
            .await;
    }

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use iso8601_timestamp::{Duration, Timestamp};
    use revolt_database::EmailVerification;
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{ContentType, Status};

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (mut account, _, _) = harness.new_user().await;

        account.verification = EmailVerification::Pending {
            token: "token".into(),
            expiry: Timestamp::now_utc() + Duration::seconds(100),
        };

        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/account/verify/token")
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);

        // Make sure it was used and can't be used again
        assert!(
            harness
                .db
                .fetch_account_with_email_verification("token")
                .await
                .is_err()
        );

        // Check that we can login using the received MFA ticket
        let response = res
            .into_json::<v0::ResponseVerify>()
            .await
            .expect("`ResponseVerify`");

        if let v0::ResponseVerify::WithTicket { ticket } = response {
            let res = harness
                .client
                .post("/auth/session/login")
                .header(ContentType::JSON)
                .body(json!({ "mfa_ticket": ticket.token }).to_string())
                .dispatch()
                .await;

            assert_eq!(res.status(), Status::Ok);
            assert!(res.into_json::<v0::Session>().await.is_some());
        } else {
            panic!("Expected `ResponseVerify::WithTicket`");
        }
    }

    #[rocket::async_test]
    async fn fail_invalid_token() {
        let harness = TestHarness::new().await;

        let res = harness
            .client
            .post("/auth/account/verify/token")
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidToken,
        ));
    }
}
