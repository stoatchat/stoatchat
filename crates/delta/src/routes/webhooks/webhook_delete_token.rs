use revolt_database::{util::reference::Reference, Database};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Deletes a webhook
///
/// Deletes a webhook with a token
#[openapi(tag = "Webhooks")]
#[delete("/<webhook_id>/<token>")]
pub async fn webhook_delete_token(
    db: &State<Database>,
    webhook_id: Reference<'_>,
    token: String,
) -> Result<EmptyResponse> {
    let webhook = webhook_id.as_webhook(db).await?;
    webhook.assert_token(&token)?;
    webhook.delete(db).await.map(|_| EmptyResponse)
}
