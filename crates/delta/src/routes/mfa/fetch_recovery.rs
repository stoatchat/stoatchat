//! Fetch recovery codes for an account.
//! POST /mfa/recovery
use rocket::serde::json::Json;
use revolt_database::{Account, ValidatedTicket};
use revolt_result::Result;

/// # Fetch Recovery Codes
///
/// Fetch recovery codes for an account.
#[openapi(tag = "MFA")]
#[post("/recovery")]
pub async fn fetch_recovery(
    account: Account,
    _ticket: ValidatedTicket,
) -> Result<Json<Vec<String>>> {
    Ok(Json(account.mfa.recovery_codes))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::MFATicket;
    use rocket::http::{ContentType, Header, Status};


    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let ticket = MFATicket::new(account.id, true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/mfa/recovery")
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .header(ContentType::JSON)
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(res.into_json::<Vec<String>>().await.unwrap().is_empty());
    }
}
