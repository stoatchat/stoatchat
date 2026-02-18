//! Confirm an account deletion.
//! PUT /account/delete
use revolt_models::v0;
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_result::Result;
use revolt_database::Database;



/// # Confirm Account Deletion
///
/// Schedule an account for deletion by confirming the received token.
#[openapi(tag = "Account")]
#[put("/delete", data = "<data>")]
pub async fn confirm_deletion(
    db: &State<Database>,
    data: Json<v0::DataAccountDeletion>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    // Find the relevant account
    let mut account = db
        .fetch_account_with_deletion_token(&data.token)
        .await?;

    // Schedule the account for deletion
    account
        .schedule_deletion(db)
        .await
        .map(|_| EmptyResponse)
}

// #[cfg(test)]
// mod tests {
//     use chrono::Duration;
//     use iso8601_timestamp::Timestamp;

//     use crate::test::*;

//     #[async_std::test]
//     async fn success() {
//         let (authifier, _, mut account, _) =
//             for_test_authenticated("confirm_deletion::success").await;

//         account.deletion = Some(DeletionInfo::WaitingForVerification {
//             token: "token".into(),
//             expiry: Timestamp::from_unix_timestamp_ms(
//                 chrono::Utc::now()
//                     .checked_add_signed(Duration::seconds(100))
//                     .expect("failed to checked_add_signed")
//                     .timestamp_millis(),
//             ),
//         });

//         account.save(&authifier).await.unwrap();

//         let client = bootstrap_rocket_with_auth(
//             authifier,
//             routes![crate::routes::account::confirm_deletion::confirm_deletion,],
//         )
//         .await;

//         let res = client
//             .put("/delete")
//             .header(ContentType::JSON)
//             .body(
//                 json!({
//                     "token": "token"
//                 })
//                 .to_string(),
//             )
//             .dispatch()
//             .await;

//         assert_eq!(res.status(), Status::NoContent);
//     }
// }
