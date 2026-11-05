//! Begin registering a new passkey.
//! POST /mfa/passkeys
use revolt_database::{Account, Database, ValidatedTicket};
use revolt_models::v0;
use revolt_result::Result;
use rocket::State;
use rocket::serde::json::Json;

/// # Begin Passkey Registration
#[openapi(tag = "MFA")]
#[post("/passkeys")]
pub async fn passkey_register_start(
    db: &State<Database>,
    account: Account,
    _ticket: ValidatedTicket,
) -> Result<Json<v0::ResponsePasskeyRegistration>> {
    let (challenge, options) = account.start_passkey_registration(db).await?;
    Ok(Json(v0::ResponsePasskeyRegistration { challenge, options }))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::MFATicket;
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/mfa/passkeys")
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let response = res
            .into_json::<v0::ResponsePasskeyRegistration>()
            .await
            .unwrap();

        let selection = &response.options["publicKey"]["authenticatorSelection"];
        assert_eq!(selection["residentKey"], "preferred");
        assert_eq!(selection["requireResidentKey"], false);
    }

    #[rocket::async_test]
    async fn fail_without_ticket() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness
            .client
            .post("/auth/mfa/passkeys")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
    }
}
