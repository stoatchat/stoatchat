//! Generate a new secret for TOTP.
//! POST /mfa/totp
use revolt_database::{Database, Account};
use revolt_models::v0;
use revolt_result::Result;
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Enable TOTP 2FA
///
/// Generate a new secret for TOTP.
#[openapi(tag = "MFA")]
#[put("/totp", data = "<data>")]
pub async fn totp_enable(
    db: &State<Database>,
    mut account: Account,
    data: Json<v0::MFAResponse>,
) -> Result<EmptyResponse> {
    // Enable TOTP 2FA
    account.mfa.enable_totp(data.into_inner())?;

    // Save model to database
    account.save(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{MFATicket, Totp};
    use rocket::http::{ContentType, Header, Status};
    use revolt_models::v0;

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/mfa/totp")
            .header(Header::new("X-Session-Token", session.token.clone()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);

        let secret = res.into_json::<v0::ResponseTotpSecret>().await.unwrap().secret;

        let code = Totp::Enabled { secret }.generate_code().unwrap();

        let res = harness.client
            .put("/auth/mfa/totp")
            .header(Header::new("X-Session-Token", session.token))
            .header(ContentType::JSON)
            .body(json!({ "totp_code": code }).to_string())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let res = harness.client
            .post("/auth/session/login")
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": account.email.clone(),
                    "password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        let response = res.into_json::<v0::ResponseLogin>().await.unwrap();

        if let v0::ResponseLogin::MFA { ticket, .. } = response {
            let res = harness.client
                .post("/auth/session/login")
                .header(ContentType::JSON)
                .body(
                    json!({
                        "mfa_ticket": ticket,
                        "mfa_response": {
                            "totp_code": code
                        }
                    })
                    .to_string(),
                )
                .dispatch()
                .await;

            assert_eq!(res.status(), Status::Ok);
        } else {
            unreachable!("Did not receive MFA challenge!");
        }
    }
}
