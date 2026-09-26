use revolt_config::config;
use revolt_database::{util::reference::Reference, Database, DiscoverRequestType, User};
use rocket_empty::EmptyResponse;

use revolt_result::{create_error, Result};
use rocket::State;

/// # Add server to Discover
///
/// This puts your server into the Discover request queue.
/// This endpoint is ONLY USEFUL in production on stoat.chat/app .
#[openapi(tag = "Discover")]
#[put("/<server>/discover")]
pub async fn discover_add(
    db: &State<Database>,
    server: Reference<'_>,
    user: User,
) -> Result<EmptyResponse> {
    let config = config().await;
    if !config.production {
        return Err(create_error!(NoEffect));
    }

    let server = server.as_server(db).await?;
    if server.owner != user.id && !user.privileged {
        return Err(create_error!(NotOwner));
    }

    if db
        .get_discover_ban(DiscoverRequestType::Server, &server.id)
        .await
        .is_ok()
    {
        return Err(create_error!(Banned));
    }

    db.insert_discover_request(DiscoverRequestType::Server, &server.id)
        .await?;

    Ok(EmptyResponse)
}
