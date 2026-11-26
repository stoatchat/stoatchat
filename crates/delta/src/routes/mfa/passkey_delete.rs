//! Remove a passkey.
//! DELETE /mfa/passkeys/:id
use revolt_database::{Account, Database, ValidatedTicket};
use revolt_result::{Result, create_error};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Delete Passkey
///
/// Remove a registered passkey
#[openapi(tag = "MFA")]
#[delete("/passkeys/<id>")]
pub async fn passkey_delete(
    db: &State<Database>,
    mut account: Account,
    _ticket: ValidatedTicket,
    id: String,
) -> Result<EmptyResponse> {
    let index = account
        .mfa
        .passkeys
        .iter()
        .position(|passkey| passkey.id == id)
        .ok_or_else(|| create_error!(NotFound))?;

    account.mfa.passkeys.remove(index);
    account.save(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{
        rocket,
        util::{passkey::register_passkey, test::TestHarness},
    };
    use revolt_database::MFATicket;
    use rocket::http::{Header, Status};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let passkey = register_passkey(&harness, &account, &session, &mut authenticator).await;

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .delete(format!("/auth/mfa/passkeys/{}", passkey.id))
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        assert!(account.mfa.passkeys.is_empty());
        assert!(!account.mfa.is_active());
    }

    #[rocket::async_test]
    async fn fail_without_ticket() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let passkey = register_passkey(&harness, &account, &session, &mut authenticator).await;

        let res = harness
            .client
            .delete(format!("/auth/mfa/passkeys/{}", passkey.id))
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        assert_eq!(account.mfa.passkeys.len(), 1);
    }
}
