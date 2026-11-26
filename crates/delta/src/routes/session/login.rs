//! Login to an account
//! POST /session/login
use std::time::Duration;

use iso8601_timestamp::Timestamp;
use revolt_database::{
    Account, Database, EmailVerification, MFATicket, PartialAccount,
    util::{email::normalise_email, password::assert_safe},
};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::State;
use rocket::serde::json::Json;
use tokio::time::sleep;

/// # Login
///
/// Login to an account.
#[openapi(tag = "Session")]
#[post("/login", data = "<data>")]
pub async fn login(
    db: &State<Database>,
    data: Json<v0::DataLogin>,
) -> Result<Json<v0::ResponseLogin>> {
    // Random jitter from 0-1000ms
    sleep(Duration::from_millis(
        (rand::random::<f32>() * 1000.) as u64,
    ))
    .await;

    let (account, name) = match data.into_inner() {
        v0::DataLogin::Email {
            email,
            password,
            friendly_name,
        } => {
            // Try to find the account we want
            let email_normalised = normalise_email(email);

            // Lookup the email in database
            if let Some(mut account) = db
                .fetch_account_by_normalised_email(&email_normalised)
                .await?
            {
                // Make sure the account has been verified
                if let EmailVerification::Pending { .. } = account.verification {
                    return Err(create_error!(UnverifiedAccount));
                }

                // Make sure password has not been compromised
                assert_safe(&password).await?;

                // Check for account lockout
                if let Some(lockout) = &account.lockout {
                    if let Some(expiry) = lockout.expiry {
                        if expiry > Timestamp::now_utc() {
                            db.bump_lockout_count(&account.id).await?;
                            return Err(create_error!(LockedOut));
                        }
                    }
                }

                // Verify the password is correct.
                if let Err(err) = account.verify_password(&password) {
                    db.bump_lockout_count(&account.id).await?;
                    return Err(err);
                }

                // Check whether an MFA step is required
                if account.mfa.is_active() {
                    // Create a new ticket
                    let mut ticket = MFATicket::new(account.id, false);
                    ticket.populate(&account.mfa).await;
                    ticket.save(db).await?;

                    // Return applicable methods
                    return Ok(Json(v0::ResponseLogin::MFA {
                        ticket: ticket.token,
                        allowed_methods: account
                            .mfa
                            .get_methods()
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    }));
                }

                // No MFA, clear lockout information if present
                if account.lockout.is_some() {
                    account.lockout = None;
                    account.save(db).await?;
                }

                (account, friendly_name)
            } else {
                return Err(create_error!(InvalidCredentials));
            }
        }
        v0::DataLogin::MFA {
            mfa_ticket,
            mfa_response,
            friendly_name,
        } => {
            // Resolve the MFA ticket
            let ticket = &mut db.fetch_ticket_by_token(&mfa_ticket).await?;

            // Find the corresponding account
            let mut account = db.fetch_account(&ticket.account_id).await?;

            // Check for account lockout
            if let Some(lockout) = &account.lockout
                && let Some(expiry) = lockout.expiry
                && expiry > Timestamp::now_utc()
            {
                db.bump_lockout_count(&account.id).await?;
                return Err(create_error!(LockedOut));
            }

            // Verify the MFA response
            if let Some(mfa_response) = mfa_response {
                if let Err(err) = account
                    .consume_mfa_response(db, mfa_response, Some(ticket))
                    .await
                {
                    // MFA failures count towards lockout
                    db.bump_lockout_count(&account.id).await?;
                    return Err(err);
                }
            } else if !ticket.authorised {
                return Err(create_error!(InvalidToken));
            }

            // MFA passed, clear lockout information if present
            if account.lockout.is_some() {
                account.lockout = None;
                account.save(db).await?;
            }

            (account, friendly_name)
        }
        v0::DataLogin::Passkey {
            passkey_challenge,
            passkey_response,
            friendly_name,
        } => {
            // Find the account this passkey belongs to
            let (mut account, passkey_login) =
                Account::identify_passkey_login(db, &passkey_challenge, passkey_response).await?;

            // Check for account lockout
            if let Some(lockout) = &account.lockout
                && let Some(expiry) = lockout.expiry
                && expiry > Timestamp::now_utc()
            {
                db.bump_lockout_count(&account.id).await?;
                return Err(create_error!(LockedOut));
            }

            // Verify the passkey, user verification makes this sufficient on its own
            if let Err(err) = account.finish_passkey_login(db, passkey_login).await {
                db.bump_lockout_count(&account.id).await?;
                return Err(err);
            }

            // Passkey passed, clear lockout information and save passkey usage
            account.lockout = None;
            account.save(db).await?;

            (account, friendly_name)
        }
    };

    // Generate a session name
    let name = name.unwrap_or_else(|| "Unknown".to_string());

    // Prevent disabled accounts from logging in
    if account.disabled {
        return Ok(Json(v0::ResponseLogin::Disabled {
            user_id: account.id,
        }));
    }

    // Create and return a new session
    Ok(Json(v0::ResponseLogin::Success(
        account.create_session(db, name).await?.into(),
    )))
}

#[cfg(test)]
mod tests {
    use crate::util::{
        passkey::{register_passkey, sign_challenge, sign_login_challenge},
        test::PubSubTestHelper,
    };
    use crate::{rocket, util::test::TestHarness};
    use iso8601_timestamp::Timestamp;
    use revolt_database::{
        Account, EmailVerification, Lockout, MFATicket, Totp, events::client::EventV1,
        util::webauthn::encode_credential_id,
    };
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{ContentType, Header, Status};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;

        let account = Account::new(
            &harness.db,
            "example@validemail.com".into(),
            "password_insecure".into(),
            false,
        )
        .await
        .unwrap();

        let mut pubsub = PubSubTestHelper::new("global").await;

        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "EXAMPLE@validemail.com",
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(res.into_json::<v0::Session>().await.is_some());

        let event = pubsub
            .wait_for_event(|event| match event {
                EventV1::CreateSession { session } => session.user_id == account.id,
                _ => false,
            })
            .await;
        if !matches!(event, EventV1::CreateSession { .. }) {
            panic!("Received incorrect event type. {:?}", event);
        }
    }

    #[rocket::async_test]
    async fn success_totp_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, _, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": &account.email,
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let response = serde_json::from_str::<v0::ResponseLogin>(&res.into_string().await.unwrap())
            .expect("`ResponseLogin`");

        if let v0::ResponseLogin::MFA {
            ticket,
            allowed_methods,
        } = response
        {
            assert!(allowed_methods.contains(&v0::MFAMethod::Totp));

            let res = harness
                .client
                .post("/auth/session/login")
                .header(ContentType::JSON)
                .body(
                    json!({
                        "mfa_ticket": ticket,
                        "mfa_response": {
                            "totp_code": totp.generate_code().expect("totp code")
                        }
                    })
                    .to_string(),
                )
                .dispatch()
                .await;

            assert_eq!(res.status(), Status::Ok);
            assert!(serde_json::from_str::<v0::Session>(&res.into_string().await.unwrap()).is_ok());
        } else {
            panic!("expected `ResponseLogin::MFA`")
        }
    }

    #[rocket::async_test]
    async fn success_totp_stored_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, _, _) = harness.new_user().await;

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
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "mfa_ticket": ticket.token,
                    "mfa_response": {
                        "totp_code": "token from earlier"
                    }
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(serde_json::from_str::<v0::Session>(&res.into_string().await.unwrap()).is_ok());
    }

    #[rocket::async_test]
    async fn fail_totp_invalid_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, _, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "email": account.email.clone(),
                "password": "password_insecure"
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let response = serde_json::from_str::<v0::ResponseLogin>(&res.into_string().await.unwrap())
            .expect("`ResponseLogin`");

        if let v0::ResponseLogin::MFA {
            ticket,
            allowed_methods,
        } = response
        {
            assert!(allowed_methods.contains(&v0::MFAMethod::Totp));

            let res = harness
                .client
                .post("/auth/session/login")
                .json(&json!({
                    "mfa_ticket": ticket,
                    "mfa_response": {
                        "totp_code": "some random data"
                    }
                }))
                .dispatch()
                .await;

            assert_eq!(res.status(), Status::Unauthorized);
            assert!(matches!(
                res.into_json::<Error>().await.unwrap().error_type,
                ErrorType::InvalidToken,
            ));
        } else {
            panic!("expected `ResponseLogin::MFA`")
        }
    }

    /// Fetch a passwordless login challenge
    async fn passkey_login_challenge(harness: &TestHarness) -> v0::ResponsePasskeyChallenge {
        harness
            .client
            .post("/auth/session/login/passkey")
            .dispatch()
            .await
            .into_json::<v0::ResponsePasskeyChallenge>()
            .await
            .unwrap()
    }

    #[rocket::async_test]
    async fn success_passkey_mfa() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "email": &account.email,
                "password": "password_insecure"
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let v0::ResponseLogin::MFA {
            ticket,
            allowed_methods,
        } = res.into_json::<v0::ResponseLogin>().await.unwrap()
        else {
            panic!("expected `ResponseLogin::MFA`")
        };

        assert!(allowed_methods.contains(&v0::MFAMethod::Passkey));

        let challenge = harness
            .client
            .post("/auth/mfa/passkeys/challenge")
            .header(Header::new("X-MFA-Ticket", ticket.clone()))
            .dispatch()
            .await
            .into_json::<v0::ResponsePasskeyChallenge>()
            .await
            .unwrap();

        let response = sign_challenge(&mut authenticator, challenge.options).await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "mfa_ticket": ticket,
                "mfa_response": {
                    "passkey_challenge": challenge.challenge,
                    "passkey_response": response
                }
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(serde_json::from_str::<v0::Session>(&res.into_string().await.unwrap()).is_ok());

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        assert!(account.mfa.passkeys[0].last_used.is_some());
    }

    #[rocket::async_test]
    async fn fail_password_mfa_with_passkey() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "email": &account.email,
                "password": "password_insecure"
            }))
            .dispatch()
            .await;

        let v0::ResponseLogin::MFA { ticket, .. } =
            res.into_json::<v0::ResponseLogin>().await.unwrap()
        else {
            panic!("expected `ResponseLogin::MFA`")
        };

        // A password can't stand in as the second factor
        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "mfa_ticket": ticket,
                "mfa_response": {
                    "password": "password_insecure"
                }
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::BadRequest);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::DisallowedMFAMethod,
        ));
    }

    #[rocket::async_test]
    async fn success_passkey_login() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        // TOTP is not required when logging in with a passkey
        account = harness.db.fetch_account(&account.id).await.unwrap();
        account.mfa.totp_token = Totp::Enabled {
            secret: "secret".to_string(),
        };
        account.save(&harness.db).await.unwrap();

        let challenge = passkey_login_challenge(&harness).await;
        let response =
            sign_login_challenge(&harness, &account, &mut authenticator, challenge.options).await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "passkey_challenge": challenge.challenge,
                "passkey_response": response
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let session = serde_json::from_str::<v0::Session>(&res.into_string().await.unwrap())
            .expect("`Session`");
        assert_eq!(session.user_id, account.id);
    }

    #[rocket::async_test]
    async fn fail_passkey_login_reused_challenge() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let challenge = passkey_login_challenge(&harness).await;
        let response =
            sign_login_challenge(&harness, &account, &mut authenticator, challenge.options).await;

        let body = json!({
            "passkey_challenge": challenge.challenge,
            "passkey_response": response
        });

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&body)
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&body)
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidToken,
        ));
    }

    #[rocket::async_test]
    async fn fail_passkey_login_wrong_user_handle() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;
        let (other_account, _, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let challenge = passkey_login_challenge(&harness).await;
        let mut response =
            sign_login_challenge(&harness, &account, &mut authenticator, challenge.options).await;

        response["response"]["userHandle"] = serde_json::Value::String(encode_credential_id(
            other_account.id_as_uuid().unwrap().as_bytes(),
        ));

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "passkey_challenge": challenge.challenge,
                "passkey_response": response
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));
    }

    #[rocket::async_test]
    async fn fail_passkey_login_bad_signature() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        // Sign a different challenge than the one we submit against
        let signed = passkey_login_challenge(&harness).await;
        let response =
            sign_login_challenge(&harness, &account, &mut authenticator, signed.options).await;
        let challenge = passkey_login_challenge(&harness).await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "passkey_challenge": challenge.challenge,
                "passkey_response": response
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));

        // Failure counts towards lockout
        let account = harness.db.fetch_account(&account.id).await.unwrap();
        assert_eq!(account.lockout.unwrap().attempts, 1);
    }

    #[rocket::async_test]
    async fn fail_passkey_login_disabled_account() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let mut account = harness.db.fetch_account(&account.id).await.unwrap();
        account.disabled = true;
        account.save(&harness.db).await.unwrap();

        let challenge = passkey_login_challenge(&harness).await;
        let response =
            sign_login_challenge(&harness, &account, &mut authenticator, challenge.options).await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "passkey_challenge": challenge.challenge,
                "passkey_response": response
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(matches!(
            res.into_json::<v0::ResponseLogin>().await.unwrap(),
            v0::ResponseLogin::Disabled { .. }
        ));
    }

    #[rocket::async_test]
    async fn fail_invalid_user() {
        let harness = TestHarness::new().await;

        let res = harness
            .client
            .post("/auth/session/login")
            .json(&json!({
                "email": "example@validemail.com",
                "password": "password_insecure"
            }))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));
    }

    #[rocket::async_test]
    async fn fail_disabled_account() {
        let harness = TestHarness::new().await;

        let mut account = Account::new(
            &harness.db,
            "example@validemail.com".into(),
            "password_insecure".into(),
            false,
        )
        .await
        .unwrap();

        account.disabled = true;
        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let response = serde_json::from_str::<v0::ResponseLogin>(&res.into_string().await.unwrap())
            .expect("`ResponseLogin`");

        assert!(matches!(response, v0::ResponseLogin::Disabled { .. }));
    }

    #[rocket::async_test]
    async fn fail_unverified_account() {
        let harness = TestHarness::new().await;

        let mut account = Account::new(
            &harness.db,
            "example@validemail.com".into(),
            "password_insecure".into(),
            false,
        )
        .await
        .unwrap();

        account.verification = EmailVerification::Pending {
            token: "".to_string(),
            expiry: Timestamp::now_utc(),
        };

        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Forbidden);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::UnverifiedAccount,
        ));
    }

    #[rocket::async_test]
    async fn fail_locked_account() {
        let harness = TestHarness::new().await;

        let mut account = Account::new(
            &harness.db,
            "example@validemail.com".into(),
            "password_insecure".into(),
            false,
        )
        .await
        .unwrap();

        account.save(&harness.db).await.unwrap();

        // Attempt 1
        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "wrong_password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));

        // Attempt 2
        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "wrong_password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));

        // Attempt 3
        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "wrong_password"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));

        // Attempt 4: Locked Out
        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Forbidden);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::LockedOut,
        ));
        // Pretend it expired
        account.lockout = Some(Lockout {
            attempts: 9001,
            expiry: Some(Timestamp::now_utc()),
        });

        account.save(&harness.db).await.unwrap();

        // Once it expires, we can log in.
        let res = harness
            .client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example@validemail.com",
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(serde_json::from_str::<v0::Session>(&res.into_string().await.unwrap()).is_ok());
    }
}
