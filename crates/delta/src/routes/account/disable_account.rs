//! Disable an account.
//! POST /account/disable
use revolt_result::{create_error, Result};
use revolt_database::{Database, Account, ValidatedTicket};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Disable Account
///
/// Disable an account.
#[openapi(tag = "Account")]
#[post("/disable")]
pub async fn disable_account(
    db: &State<Database>,
    mut account: Account,
    validated_ticket: Option<ValidatedTicket>,
) -> Result<EmptyResponse>
{
    if account.mfa.is_active() && validated_ticket.is_none() {
        return Err(create_error!(InvalidCredentials));
    }

    account.disable(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{events::client::EventV1, MFATicket, Totp};
    use revolt_result::ErrorType;
    use rocket::http::{Header, Status};
    use crate::util::test::PubSubTestHelper;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;
        let mut pubsub = PubSubTestHelper::new(&format!("{}!", &account.id)).await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "disable_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/account/disable")
            .header(Header::new("X-Session-Token", session.token.clone()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
        drop(res);
        assert!(
            harness.db
                .fetch_account(&account.id)
                .await
                .unwrap()
                .disabled
        );

        assert!(matches!(
            harness.db
                .fetch_session(&session.id)
                .await
                .unwrap_err().error_type,
            ErrorType::UnknownUser
        ));

        pubsub.wait_for_event(|e| if let EventV1::DeleteAllSessions { user_id, .. } = e {
            user_id == &account.id
        } else {
            false
        }).await;
    }

    #[rocket::async_test]
    async fn fail_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "disable_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/account/disable")
            .header(Header::new("X-Session-Token", session.token.clone()))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
    }
}
