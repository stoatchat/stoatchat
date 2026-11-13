//! Finish registering a new passkey.
//! PUT /mfa/passkeys
use revolt_database::{Account, Database};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::State;
use rocket::serde::json::Json;
use validator::Validate;

/// # Finish Passkey Registration
///
/// Finish registering a new passkey with the result from the client.
#[openapi(tag = "MFA")]
#[put("/passkeys", data = "<data>")]
pub async fn passkey_register_finish(
    db: &State<Database>,
    mut account: Account,
    data: Json<v0::DataRegisterPasskey>,
) -> Result<Json<v0::PasskeyInfo>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    account
        .finish_passkey_registration(db, &data.challenge, data.name, data.credential)
        .await
        .map(|passkey| Json(passkey.into()))
}

#[cfg(test)]
mod tests {
    use crate::{
        rocket,
        util::{passkey::register_passkey, test::TestHarness},
    };
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{ContentType, Header, Status};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let passkey = register_passkey(&harness, &account, &session, &mut authenticator).await;
        assert_eq!(passkey.name, "Test Passkey");

        // The software authenticator doesn't report `credProps`
        assert!(!passkey.discoverable);

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        assert_eq!(account.mfa.passkeys.len(), 1);
        assert!(account.mfa.is_active());
    }

    #[rocket::async_test]
    async fn success_records_discoverable() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let (challenge, mut credential) = crate::util::passkey::create_credential(
            &harness,
            &account,
            &session,
            &mut authenticator,
        )
        .await;

        // Browsers report whether they created a resident key through `credProps`
        credential["extensions"]["credProps"] = json!({ "rk": true });

        let res = harness
            .client
            .put("/auth/mfa/passkeys")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token))
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
        assert!(
            res.into_json::<v0::PasskeyInfo>()
                .await
                .unwrap()
                .discoverable
        );
    }

    #[rocket::async_test]
    async fn fail_credential_on_two_accounts() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;
        let (other_account, _, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        let mut other_account = harness.db.fetch_account(&other_account.id).await.unwrap();
        other_account
            .mfa
            .passkeys
            .push(account.mfa.passkeys[0].clone());

        // Rejected by the unique `mfa.passkeys.credential_id` index
        assert!(other_account.save(&harness.db).await.is_err());
    }

    #[rocket::async_test]
    async fn fail_reused_challenge() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let (challenge, credential) = crate::util::passkey::create_credential(
            &harness,
            &account,
            &session,
            &mut authenticator,
        )
        .await;

        let body = json!({
            "challenge": challenge,
            "name": "Test Passkey",
            "credential": credential
        })
        .to_string();

        let res = harness
            .client
            .put("/auth/mfa/passkeys")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token.clone()))
            .body(&body)
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        let res = harness
            .client
            .put("/auth/mfa/passkeys")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token))
            .body(&body)
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidToken,
        ));
    }

    #[rocket::async_test]
    async fn fail_other_accounts_challenge() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;
        let (_, other_session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let (challenge, credential) = crate::util::passkey::create_credential(
            &harness,
            &account,
            &session,
            &mut authenticator,
        )
        .await;

        let res = harness
            .client
            .put("/auth/mfa/passkeys")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", other_session.token))
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

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidToken,
        ));
    }
}
