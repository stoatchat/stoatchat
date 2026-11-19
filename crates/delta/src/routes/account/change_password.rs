//! Change account password.
//! PATCH /account/change/password
use revolt_database::{
    AMQP, Account, Database, User, ValidatedTicket,
    util::password::{assert_safe, hash_password},
};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::State;
use rocket::serde::json::Json;
use rocket_empty::EmptyResponse;

/// # Change Password
///
/// Change the current account password.
#[openapi(tag = "Account")]
#[patch("/change/password", data = "<data>")]
pub async fn change_password(
    db: &State<Database>,
    amqp: &State<AMQP>,
    validated_ticket: Option<ValidatedTicket>,
    mut account: Account,
    user: User,
    data: Json<v0::DataChangePassword>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    if account.mfa.is_active() && validated_ticket.is_none() {
        return Err(create_error!(InvalidCredentials));
    }

    // Verify password can be used
    assert_safe(&data.password).await?;

    // Ensure given password is correct
    account.verify_password(&data.current_password)?;

    // Hash and replace password
    account.password = hash_password(data.password)?;

    // Commit to database
    account.save(db).await?;

    _ = user
        .send_system_notice(db, amqp, revolt_database::SystemNotice::PasswordChanged)
        .await;

    Ok(EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{MFATicket, Totp};
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness
            .client
            .patch("/auth/account/change/password")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token.clone()))
            .body(
                json!({
                    "password": "new password",
                    "current_password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let res = harness
            .client
            .patch("/auth/account/change/password")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token))
            .body(
                json!({
                    "password": "sussy password",
                    "current_password": "new password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }

    #[rocket::async_test]
    async fn success_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .patch("/auth/account/change/password")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token.clone()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .body(
                json!({
                    "password": "new password",
                    "current_password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }

    #[rocket::async_test]
    async fn fail_mfa_no_ticket() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .patch("/auth/account/change/password")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token.clone()))
            .body(
                json!({
                    "password": "new password",
                    "current_password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn fail_mfa_invalid_password() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let mut ticket = MFATicket::new(account.id.to_string(), true);
        ticket.last_totp_code = Some("token from earlier".into());
        ticket.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .patch("/auth/account/change/password")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token.clone()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .body(
                json!({
                    "password": "new password",
                    "current_password": "incorrect password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
    }
}
