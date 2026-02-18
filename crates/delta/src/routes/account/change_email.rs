//! Change account email.
//! PATCH /account/change/email
use revolt_database::util::email::validate_email;
use revolt_database::{Database, Account};
use revolt_models::v0;
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_result::Result;

/// # Change Email
///
/// Change the associated account email.
#[openapi(tag = "Account")]
#[patch("/change/email", data = "<data>")]
pub async fn change_email(
    db: &State<Database>,
    mut account: Account,
    data: Json<v0::DataChangeEmail>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    validate_email(&data.email)?;

    // Ensure given password is correct
    account.verify_password(&data.current_password)?;

    // Send email verification for new email
    account
        .start_email_move(db, data.email)
        .await
        .map(|_| EmptyResponse)
}

// #[cfg(test)]
// mod tests {
//     use crate::{routes::account::verify_email::ResponseVerify, test::*};

//     #[async_std::test]
//     async fn success() {
//         use rocket::http::Header;

//         let (authifier, session, account, _) =
//             for_test_authenticated("change_email::success").await;
//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![crate::routes::account::change_email::change_email],
//         )
//         .await;

//         let res = client
//             .patch("/change/email")
//             .header(ContentType::JSON)
//             .header(Header::new("X-Session-Token", session.token.clone()))
//             .body(
//                 json!({
//                     "email": "validexample@valid.com",
//                     "current_password": "password_insecure"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let account = authifier.database.find_account(&account.id).await.unwrap();

//         assert_eq!(account.email, "validexample@valid.com");
//     }

//     #[async_std::test]
//     async fn success_smtp() {
//         use rocket::http::Header;

//         let (authifier, session, account, _) = for_test_authenticated_with_config(
//             "change_email::success_smtp",
//             test_smtp_config().await,
//         )
//         .await;
//         let client = bootstrap_rocket_with_auth(
//             authifier.clone(),
//             routes![
//                 crate::routes::account::change_email::change_email,
//                 crate::routes::account::verify_email::verify_email
//             ],
//         )
//         .await;

//         let res = client
//             .patch("/change/email")
//             .header(ContentType::JSON)
//             .header(Header::new("X-Session-Token", session.token.clone()))
//             .body(
//                 json!({
//                     "email": "change_email@smtp.test",
//                     "current_password": "password_insecure"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);

//         let mail = assert_email_sendria("change_email@smtp.test".into()).await;
//         let res = client
//             .post(format!("/verify/{}", mail.code.expect("`code`")))
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::Ok);

//         let account = authifier.database.find_account(&account.id).await.unwrap();

//         assert_eq!(account.email, "change_email@smtp.test");

//         // Ensure that we did not receive a ticket
//         assert_eq!(
//             ResponseVerify::NoTicket,
//             serde_json::from_str::<crate::routes::account::verify_email::ResponseVerify>(
//                 &res.into_string().await.unwrap(),
//             )
//             .expect("`ResponseVerify`")
//         )
//     }
// }
