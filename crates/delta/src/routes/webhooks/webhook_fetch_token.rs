use revolt_database::{util::reference::Reference, Database};
use revolt_models::v0::Webhook;
use revolt_result::Result;
use rocket::{serde::json::Json, State};

/// # Gets a webhook
///
/// Gets a webhook with a token
#[openapi(tag = "Webhooks")]
#[get("/<webhook_id>/<token>")]
pub async fn webhook_fetch_token(
    db: &State<Database>,
    webhook_id: Reference<'_>,
    token: String,
) -> Result<Json<Webhook>> {
    let webhook = webhook_id.as_webhook(db).await?;
    webhook.assert_token(&token)?;
    Ok(Json(webhook.into()))
}
