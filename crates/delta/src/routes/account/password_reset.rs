//! Confirm a password reset.
//! PATCH /account/reset_password
use revolt_database::util::password::{assert_safe, hash_password};
use revolt_database::{AMQP, Database};
use revolt_models::v0;
use revolt_result::Result;
use rocket::State;
use rocket::serde::json::Json;
use rocket_empty::EmptyResponse;

/// # Password Reset
///
/// Confirm password reset and change the password.
#[openapi(tag = "Account")]
#[patch("/reset_password", data = "<data>")]
pub async fn password_reset(
    db: &State<Database>,
    amqp: &State<AMQP>,
    data: Json<v0::DataPasswordReset>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    // Find the relevant account
    let mut account = db.fetch_account_with_password_reset(&data.token).await?;

    // Verify password can be used
    assert_safe(&data.password).await?;

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

    let user = db.fetch_user(&account.id).await?;
    _ = user
        .send_system_notice(db, amqp, revolt_database::SystemNotice::PasswordChanged)
        .await;

    Ok(EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use iso8601_timestamp::{Duration, Timestamp};
    use revolt_database::PasswordReset;
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{ContentType, Status};

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        account.password_reset = Some(PasswordReset {
            token: "token".into(),
            expiry: Timestamp::now_utc() + Duration::seconds(100),
        });

        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .patch("/auth/account/reset_password")
            .header(ContentType::JSON)
            .body(
                json!({
                    "token": "token",
                    "password": "valid-password",
                    "remove_sessions": true
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        // Make sure it was used and can't be used again
        assert!(
            harness
                .db
                .fetch_account_with_password_reset("token")
                .await
                .is_err()
        );

        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": account.email.clone(),
                    "password": "valid-password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(res.into_json::<v0::Session>().await.is_some());

        // Ensure sessions were deleted
        assert!(matches!(
            harness
                .db
                .fetch_session(&session.id)
                .await
                .unwrap_err()
                .error_type,
            ErrorType::UnknownUser
        ));
    }

    #[rocket::async_test]
    async fn fail_invalid_token() {
        let harness = TestHarness::new().await;

        let res = harness
            .client
            .patch("/auth/account/reset_password")
            .header(ContentType::JSON)
            .body(
                json!({
                    "token": "invalid",
                    "password": "valid password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidToken
        ));
    }
}
