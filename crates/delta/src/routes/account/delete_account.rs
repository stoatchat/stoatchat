//! Delete an account.
//! POST /account/delete
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_result::{create_error, Result};
use revolt_database::{Database, Account, ValidatedTicket};
/// # Delete Account
///
/// Request to have an account deleted.
#[openapi(tag = "Account")]
#[post("/delete")]
pub async fn delete_account(
    db: &State<Database>,
    mut account: Account,
    validated_ticket: Option<ValidatedTicket>,
) -> Result<EmptyResponse> {
    if account.mfa.is_active() && validated_ticket.is_none() {
        return Err(create_error!(InvalidCredentials));
    }

    account
        .start_account_deletion(db)
        .await
        .map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{MFATicket, Totp};
    use rocket::http::{ContentType, Header, Status};
    use revolt_config::overwrite_config;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "delete_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/account/delete")
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let (_, code) = harness.assert_email("delete_account@smtp.test").await;
        let res = harness.client
            .put("/auth/account/delete")
            .header(ContentType::JSON)
            .body(
                json!({
                    "token": code
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }

    #[rocket::async_test]
    async fn fail_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "delete_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/account/delete")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
    }
}
