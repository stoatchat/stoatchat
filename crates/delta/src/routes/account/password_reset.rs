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

    println!("password {}", data.password);
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

#[cfg(test)]
mod tests {
    use iso8601_timestamp::{Timestamp, Duration};
    use revolt_database::PasswordReset;
    use revolt_models::v0;
    use revolt_result::{ErrorType, Error};
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{ContentType, Status};

    // #[async_std::test]
    // async fn success() {
    //     let harness = TestHarness::new().await;
    //     let (mut account, session, _) = harness.new_user().await;

    //     account.password_reset = Some(PasswordReset {
    //         token: "token".into(),
    //         expiry: Timestamp::now_utc() + Duration::seconds(100),
    //     });

    //     account.save(&harness.db).await.unwrap();

    //     let res = harness.client
    //         .patch("/auth/account/reset_password")
    //         .header(ContentType::JSON)
    //         .body(
    //             json!({
    //                 "token": "token",
    //                 "password": "valid-password",
    //                 "remove_sessions": true
    //             })
    //             .to_string(),
    //         )
    //         .dispatch()
    //         .await;

    //     assert_eq!(res.status(), Status::NoContent);

    //     // Make sure it was used and can't be used again
    //     assert!(harness.db
    //         .fetch_account_with_password_reset("token")
    //         .await
    //         .is_err());

    //     let res = harness.client
    //         .post("/auth/session/login")
    //         .header(ContentType::JSON)
    //         .body(
    //             json!({
    //                 "email": account.email.clone(),
    //                 "password": "valid-password"
    //             })
    //             .to_string(),
    //         )
    //         .dispatch()
    //         .await;

    //     assert_eq!(res.status(), Status::Ok);
    //     assert!(res.into_json::<v0::Session>().await.is_some());

    //     // Ensure sessions were deleted
    //     assert!(matches!(
    //         harness
    //             .db
    //             .fetch_session(&session.id)
    //             .await
    //             .unwrap_err().error_type,
    //         ErrorType::UnknownUser
    //     ));
    // }

    #[async_std::test]
    async fn fail_invalid_token() {
        let harness = TestHarness::new().await;

        let res = harness.client
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
