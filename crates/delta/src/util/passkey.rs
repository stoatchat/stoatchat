//! Helpers for driving passkey ceremonies in tests with a software authenticator
// Generated file.
use revolt_database::{Account, MFATicket, Session, util::webauthn::encode_credential_id};
use revolt_models::v0;
use rocket::http::{ContentType, Header, Status};
use serde_json::Value;
use url::Url;
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

use crate::util::test::TestHarness;

/// Origin the software authenticator pretends to be running on
pub async fn origin() -> Url {
    Url::parse(&revolt_config::config().await.hosts.app).unwrap()
}

/// Start a registration and have the authenticator create a credential,
/// returning the challenge token and the credential (not yet sent to the server)
pub async fn create_credential(
    harness: &TestHarness,
    account: &Account,
    session: &Session,
    authenticator: &mut SoftPasskey,
) -> (String, Value) {
    let ticket = MFATicket::new(account.id.to_string(), true);
    ticket.save(&harness.db).await.unwrap();

    let res = harness
        .client
        .post("/auth/mfa/passkeys")
        .header(Header::new("X-Session-Token", session.token.clone()))
        .header(Header::new("X-MFA-Ticket", ticket.token))
        .dispatch()
        .await;

    assert_eq!(res.status(), Status::Ok);
    let response = res
        .into_json::<v0::ResponsePasskeyRegistration>()
        .await
        .unwrap();

    // Resident keys are only preferred, so the software authenticator (which can't
    // store them) still registers, like a hardware key without resident key storage
    let credential = authenticator
        .do_registration(
            origin().await,
            serde_json::from_value(response.options).unwrap(),
        )
        .unwrap();

    (
        response.challenge,
        serde_json::to_value(credential).unwrap(),
    )
}

/// Register a new passkey on an account
pub async fn register_passkey(
    harness: &TestHarness,
    account: &Account,
    session: &Session,
    authenticator: &mut SoftPasskey,
) -> v0::PasskeyInfo {
    let (challenge, credential) = create_credential(harness, account, session, authenticator).await;

    let res = harness
        .client
        .put("/auth/mfa/passkeys")
        .header(ContentType::JSON)
        .header(Header::new("X-Session-Token", session.token.clone()))
        .body(
            json!({
                "challenge": challenge,
                "name": "Test Passkey",
                "credential": credential
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(res.status(), Status::Ok);
    res.into_json::<v0::PasskeyInfo>().await.unwrap()
}

/// Sign request options from an MFA passkey challenge
pub async fn sign_challenge(authenticator: &mut SoftPasskey, options: Value) -> Value {
    let response = authenticator
        .do_authentication(origin().await, serde_json::from_value(options).unwrap())
        .unwrap();

    serde_json::to_value(response).unwrap()
}

/// Sign request options from a passwordless login challenge as the given account
///
/// The software authenticator doesn't support discoverable credentials, so act
/// like one: pick the account's credential and attach its user handle
pub async fn sign_login_challenge(
    harness: &TestHarness,
    account: &Account,
    authenticator: &mut SoftPasskey,
    mut options: Value,
) -> Value {
    let account = harness.db.fetch_account(&account.id).await.unwrap();
    let credential_id = account.mfa.passkeys[0].credential.cred_id();

    options["publicKey"]["allowCredentials"] = json!([
        { "type": "public-key", "id": encode_credential_id(credential_id) }
    ]);

    let mut response = sign_challenge(authenticator, options).await;
    response["response"]["userHandle"] = Value::String(encode_credential_id(
        account.id_as_uuid().unwrap().as_bytes(),
    ));

    response
}
