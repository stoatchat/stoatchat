use revolt_database::{Database, User};
use revolt_result::Result;

use rocket::State;
use rocket_empty::EmptyResponse;

/// # Acknowledge Policy Changes
///
/// Accept/acknowledge changes to platform policy.
#[openapi(tag = "Policy")]
#[post("/acknowledge")]
pub async fn acknowledge_policy_changes(db: &State<Database>, user: User) -> Result<EmptyResponse> {
    db.acknowledge_policy_changes(&user.id).await?;
    Ok(EmptyResponse)
}
