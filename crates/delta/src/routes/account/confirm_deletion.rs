//! Confirm an account deletion.
//! PUT /account/delete
use revolt_database::Database;
use revolt_models::v0;
use revolt_result::Result;
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;

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
    let mut account = db.fetch_account_with_deletion_token(&data.token).await?;

    // Schedule the account for deletion
    account.schedule_deletion(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use iso8601_timestamp::{Duration, Timestamp};
    use revolt_database::DeletionInfo;
    use revolt_models::v0;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (mut account, _, _) = harness.new_user().await;

        account.deletion = Some(DeletionInfo::WaitingForVerification {
            token: "token".to_string(),
            expiry: Timestamp::now_utc() + Duration::seconds(100),
        });

        account.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .put("/auth/account/delete")
            .json(&v0::DataAccountDeletion {
                token: "token".to_string(),
            })
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);
    }
}
