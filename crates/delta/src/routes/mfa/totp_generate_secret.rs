//! Generate a new secret for TOTP.
//! POST /mfa/totp
use revolt_result::Result;
use revolt_models::v0;
use revolt_database::{Database, Account, ValidatedTicket};
use rocket::serde::json::Json;
use rocket::State;


/// # Generate TOTP Secret
///
/// Generate a new secret for TOTP.
#[openapi(tag = "MFA")]
#[post("/totp")]
pub async fn totp_generate_secret(
    db: &State<Database>,
    mut account: Account,
    _ticket: ValidatedTicket,
) -> Result<Json<v0::ResponseTotpSecret>> {
    // Generate a new secret
    let secret = account.mfa.generate_new_totp_secret()?;

    // Save model to database
    account.save(db).await?;

    // Send secret to user
    Ok(Json(v0::ResponseTotpSecret { secret }))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{MFATicket, Totp};
    use rocket::http::{Header, Status};
    use revolt_models::v0;

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/mfa/totp")
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);

        let secret = res.into_json::<v0::ResponseTotpSecret>().await.unwrap().secret;

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        assert_eq!(account.mfa.totp_token, Totp::Pending { secret });
    }
}
