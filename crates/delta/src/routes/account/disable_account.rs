//! Disable an account.
//! POST /account/disable
use revolt_result::Result;
use revolt_database::{Database, Account, ValidatedTicket};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Disable Account
///
/// Disable an account.
#[openapi(tag = "Account")]
#[post("/disable")]
pub async fn disable_account(
    db: &State<Database>,
    mut account: Account,
    _ticket: ValidatedTicket,
) -> Result<EmptyResponse> {
    account.disable(db).await.map(|_| EmptyResponse)
}

// #[cfg(test)]
// mod tests {
//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, account, receiver) =
//             for_test_authenticated("disable_account::success").await;
//         let ticket = MFATicket::new(account.id.to_string(), true);
//         ticket.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![crate::routes::account::disable_account::disable_account],
//         )
//         .await;

//         let res = client
//             .post("/disable")
//             .header(Header::new("X-Session-Token", session.token))
//             .header(Header::new("X-MFA-Ticket", ticket.token))
//             .header(ContentType::JSON)
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);
//         assert!(
//             authifier
//                 .database
//                 .find_account(&account.id)
//                 .await
//                 .unwrap()
//                 .disabled
//         );
//         assert_eq!(
//             authifier
//                 .database
//                 .find_session(&session.id)
//                 .await
//                 .unwrap_err(),
//             Error::UnknownUser
//         );

//         let event = receiver.try_recv().expect("an event");
//         if let AuthifierEvent::DeleteAllSessions { user_id, .. } = event {
//             assert_eq!(user_id, session.user_id);
//         } else {
//             panic!("Received incorrect event type. {:?}", event);
//         }
//     }
// }
