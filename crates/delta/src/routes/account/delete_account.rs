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

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, mut account, _) =
//             for_test_authenticated_with_config("delete_account::success", test_smtp_config().await)
//                 .await;

//         account.email = "delete_account@smtp.test".to_string();
//         account.save(&authifier).await.unwrap();

//         let ticket = MFATicket::new(account.id.to_string(), true);
//         ticket.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![
//                 crate::routes::account::delete_account::delete_account,
//                 crate::routes::account::confirm_deletion::confirm_deletion
//             ],
//         )
//         .await;

//         let res = client
//             .post("/delete")
//             .header(Header::new("X-Session-Token", session.token))
//             .header(Header::new("X-MFA-Ticket", ticket.token))
//             .header(ContentType::JSON)
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let mail = assert_email_sendria("delete_account@smtp.test".into()).await;
//         let res = client
//             .put("/delete")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "token": mail.code.expect("`code`")
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);
//     }
// }
