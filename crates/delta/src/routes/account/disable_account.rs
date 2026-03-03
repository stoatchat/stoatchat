//! Disable an account.
//! POST /account/disable
use revolt_result::Result;
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
    _ticket: ValidatedTicket,
) -> Result<EmptyResponse> {
    account.disable(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{MFATicket, events::client::EventV1};
    use revolt_result::ErrorType;
    use rocket::http::{Header, Status};

    #[async_std::test]
    async fn success() {
        let mut harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

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

        harness.wait_for_event(&format!("{}!", &account.id), |e| if let EventV1::DeleteAllSessions { user_id, .. } = e {
            user_id == &account.id
        } else {
            false
        }).await;
    }
}
