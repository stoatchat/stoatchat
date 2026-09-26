use revolt_config::config;
use revolt_database::{util::reference::Reference, Database, DiscoverRequestType, User};
use revolt_models::v0;

use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Get Discover request status
///
/// Fetches the status of your Discover request.
/// If it has been approved or denied, the reason will be provided (if applicable).
/// This endpoint is ONLY USEFUL in production on stoat.chat/app .
#[openapi(tag = "Discover")]
#[get("/<bot_id>/discover")]
pub async fn discover_get_bot(
    db: &State<Database>,
    bot_id: Reference<'_>,
    user: User,
) -> Result<Json<v0::DiscoverRequest>> {
    let config = config().await;
    if !config.production {
        return Err(create_error!(NoEffect));
    }

    let bot = bot_id.as_bot(db).await?;
    if (bot.owner != user.id && bot.id != user.id) && !user.privileged {
        return Err(create_error!(NotOwner));
    }

    if db
        .get_discover_ban(DiscoverRequestType::Bot, &bot.id)
        .await
        .is_ok()
    {
        return Err(create_error!(Banned));
    }

    let ret = db
        .fetch_discover_request_by_item_id(DiscoverRequestType::Bot, &bot.id)
        .await?;

    Ok(Json(ret.into()))
}
