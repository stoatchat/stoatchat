//! Login to an account
//! POST /session/login
use std::ops::Add;
use std::time::Duration;

use iso8601_timestamp::Timestamp;
use revolt_database::{
    util::{email::normalise_email, password::assert_safe},
    Database, EmailVerification, Lockout, MFATicket,
};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;

/// # Login
///
/// Login to an account.
#[openapi(tag = "Session")]
#[post("/login", data = "<data>")]
pub async fn login(
    db: &State<Database>,
    data: Json<v0::DataLogin>,
) -> Result<Json<v0::ResponseLogin>> {
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
                            return Err(create_error!(LockedOut));
                        }
                    }
                }

                // Verify the password is correct.
                if let Err(err) = account.verify_password(&password) {
                    // Lock out account if attempts are too high
                    if let Some(lockout) = &mut account.lockout {
                        lockout.attempts += 1;

                        // Allow 3 attempts
                        //
                        // Lockout for 1 minute on 3rd attempt
                        // Lockout for 5 minutes on 4th attempt
                        // Lockout for 1 hour on each subsequent attempt
                        if lockout.attempts >= 3 {
                            lockout.expiry = Some(Timestamp::now_utc().add(Duration::from_secs(
                                if lockout.attempts >= 5 {
                                    3600
                                } else if lockout.attempts == 4 {
                                    300
                                } else {
                                    60
                                },
                            )));
                        }
                    } else {
                        account.lockout = Some(Lockout {
                            attempts: 1,
                            expiry: None,
                        });
                    }

                    account.save(db).await?;
                    return Err(err);
                }

                // Clear lockout information if present
                if account.lockout.is_some() {
                    account.lockout = None;
                    account.save(db).await?;
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
            let ticket = db
                .fetch_ticket_by_token(&mfa_ticket)
                .await?
                .ok_or_else(|| create_error!(InvalidToken))?;

            // Find the corresponding account
            let mut account = db.fetch_account(&ticket.account_id).await?;

            // Verify the MFA response
            if let Some(mfa_response) = mfa_response {
                account
                    .consume_mfa_response(db, mfa_response, Some(ticket))
                    .await?;
            } else if !ticket.authorised {
                return Err(create_error!(InvalidToken));
            }

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
    use iso8601_timestamp::Timestamp;
    use revolt_database::{Account, EmailVerification, Lockout, MFATicket, Totp, events::client::EventV1};
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{ContentType, Status};
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};

    #[async_std::test]
    async fn success() {
        let mut harness = TestHarness::new().await;

        Account::new(
            &harness.db,
            "example@validemail.com".into(),
            "password_insecure".into(),
            false,
        )
        .await
        .unwrap();

        harness.wait_for_event("global", |_| true).await;

        let res = harness.client
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

        let event = harness.wait_for_event("global", |_| true).await;
        if !matches!(event, EventV1::CreateSession { .. }) {
            panic!("Received incorrect event type. {:?}", event);
        }
    }

    #[async_std::test]
    async fn success_totp_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, _, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let res = harness.client
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
        let response = serde_json::from_str::<v0::ResponseLogin>(
            &res.into_string().await.unwrap(),
        )
        .expect("`ResponseLogin`");

        if let v0::ResponseLogin::MFA {
            ticket,
            allowed_methods,
        } = response
        {
            assert!(allowed_methods.contains(&v0::MFAMethod::Totp));

            let res = harness.client
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

    #[async_std::test]
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

        let res = harness.client
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

    #[async_std::test]
    async fn fail_totp_invalid_mfa() {
        let harness = TestHarness::new().await;
        println!("{:?}", harness.db);
        let (mut account, _, _) = harness.new_user().await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/session/login")
            .json(
                &json!({
                    "email": account.email.clone(),
                    "password": "password_insecure"
                })
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let response = serde_json::from_str::<v0::ResponseLogin>(
            &res.into_string().await.unwrap(),
        )
        .expect("`ResponseLogin`");

        if let v0::ResponseLogin::MFA {
            ticket,
            allowed_methods,
        } = response
        {
            assert!(allowed_methods.contains(&v0::MFAMethod::Totp));

            let res = harness.client
                .post("/auth/session/login")
                .json(
                    &json!({
                        "mfa_ticket": ticket,
                        "mfa_response": {
                            "totp_code": "some random data"
                        }
                    })
                )
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

    #[async_std::test]
    async fn fail_invalid_user() {
        let harness = TestHarness::new().await;

        let res = harness.client
            .post("/auth/session/login")
            .json(
                &json!({
                    "email": "example@validemail.com",
                    "password": "password_insecure"
                })
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Unauthorized);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::InvalidCredentials,
        ));
    }

    #[async_std::test]
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

        let res = harness.client
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
        let response = serde_json::from_str::<v0::ResponseLogin>(
            &res.into_string().await.unwrap(),
        )
        .expect("`ResponseLogin`");

        assert!(matches!(
            response,
            v0::ResponseLogin::Disabled { .. }
        ));
    }

    #[async_std::test]
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

        let res = harness.client
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

    #[async_std::test]
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
        let res = harness.client
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
        let res = harness.client
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
        let res = harness.client
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
        let res = harness.client
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
        let res = harness.client
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
