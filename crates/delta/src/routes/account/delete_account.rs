//! Delete an account.
//! POST /account/delete
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_result::Result;
use revolt_database::{Database, Account, ValidatedTicket};
/// # Delete Account
///
/// Request to have an account deleted.
#[openapi(tag = "Account")]
#[post("/delete")]
pub async fn delete_account(
    db: &State<Database>,
    mut account: Account,
    _ticket: ValidatedTicket,
) -> Result<EmptyResponse> {
    account
        .start_account_deletion(db)
        .await
        .map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::MFATicket;
    use rocket::http::{ContentType, Header, Status};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        account.email = "delete_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let res = harness.client
            .post("/auth/account/delete")
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let (_, code) = harness.assert_email("delete_account@smtp.test").await;
        let res = harness.client
            .put("/auth/account/delete")
            .header(ContentType::JSON)
            .body(
                json!({
                    "token": code
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }
}
