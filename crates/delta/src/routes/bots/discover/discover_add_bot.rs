use revolt_config::config;
use revolt_database::{util::reference::Reference, Database, DiscoverRequestType, User};
use rocket_empty::EmptyResponse;

use revolt_result::{create_error, Result};
use rocket::State;

/// # Add bot to Discover
///
/// This puts your bot into the Discover request queue.
/// This endpoint is ONLY USEFUL in production on stoat.chat/app .
#[openapi(tag = "Discover")]
#[put("/<bot_id>/discover")]
pub async fn discover_add_bot(
    db: &State<Database>,
    bot_id: Reference<'_>,
    user: User,
) -> Result<EmptyResponse> {
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

    db.insert_discover_request(DiscoverRequestType::Bot, &bot.id)
        .await?;

    Ok(EmptyResponse)
}
