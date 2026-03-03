//! Disable TOTP 2FA.
//! DELETE /mfa/totp
use revolt_database::{Database, Account, ValidatedTicket, Totp};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Disable TOTP 2FA
///
/// Disable TOTP 2FA for an account.
#[openapi(tag = "MFA")]
#[delete("/totp")]
pub async fn totp_disable(
    db: &State<Database>,
    mut account: Account,
    _ticket: ValidatedTicket,
) -> Result<EmptyResponse> {
    // Disable TOTP
    account.mfa.totp_token = Totp::Disabled;

    // Save model to database
    account.save(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::MFATicket;
    use rocket::http::{Header, Status};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let ticket = MFATicket::new(account.id, true);
        ticket.save(&harness.db).await.unwrap();


        let res = harness.client
            .delete("/auth/mfa/totp")
            .header(Header::new("X-Session-Token", session.token.clone()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }
}
