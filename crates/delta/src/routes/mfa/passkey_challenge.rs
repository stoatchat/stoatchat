//! Create a passkey challenge to use as an MFA response.
//! POST /mfa/passkeys/challenge
use revolt_database::{Account, Database, MFATicket};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::State;
use rocket::serde::json::Json;

/// # Create Passkey Challenge
///
/// Create a challenge for one of the account's passkeys.
///
/// Use either a session token (X-Session-Token; to create an MFA ticket) or an MFA ticket (X-MFA-Ticket; to log in) as authorization
#[openapi(tag = "MFA")]
#[post("/passkeys/challenge")]
pub async fn passkey_challenge(
    db: &State<Database>,
    account: Option<Account>,
    ticket: Option<MFATicket>,
) -> Result<Json<v0::ResponsePasskeyChallenge>> {
    let account = match (account, ticket) {
        (Some(_), Some(_)) => {
            return Err(create_error!(FailedValidation {
                error: "Cannot pass both session token and MFA ticket.".to_string()
            }));
        }
        (Some(account), _) => account,
        (_, Some(ticket)) => db.fetch_account(&ticket.account_id).await?,
        _ => {
            return Err(create_error!(FailedValidation {
                error: "Pass one of X-Session-Token or X-MFA-Ticket".to_string()
            }));
        }
    };

    let (challenge, options) = account.start_passkey_authentication().await?;
    Ok(Json(v0::ResponsePasskeyChallenge { challenge, options }))
}

#[cfg(test)]
mod tests {
    use crate::{
        rocket,
        util::{passkey::register_passkey, test::TestHarness},
    };
    use revolt_database::MFATicket;
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{Header, Status};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    #[rocket::async_test]
    async fn success_with_ticket() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        register_passkey(&harness, &account, &session, &mut authenticator).await;

        let ticket = MFATicket::new(account.id.to_string(), false);
        ticket.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .post("/auth/mfa/passkeys/challenge")
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(
            res.into_json::<v0::ResponsePasskeyChallenge>()
                .await
                .is_some()
        );
    }

    #[rocket::async_test]
    async fn fail_no_passkeys() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness
            .client
            .post("/auth/mfa/passkeys/challenge")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::BadRequest);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::DisallowedMFAMethod,
        ));
    }
}
