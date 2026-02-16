use revolt_database::{Database, Member, Server, User};
use revolt_models::v0;
use revolt_result::{create_error, Result};

use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

/// # Create Server
///
/// Create a new server.
#[openapi(tag = "Server Information")]
#[post("/create", data = "<data>")]
pub async fn create_server(
    db: &State<Database>,
    user: User,
    data: Json<v0::DataCreateServer>,
) -> Result<Json<v0::CreateServerLegacyResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    user.can_acquire_server(db).await?;

    let (server, channels) = if let Some(discord_template_id) = data.discord_template_id.clone() {
        let (mut server, _) = Server::create(db, data, &user, false).await?;

        let template = revolt_discord::template::fetch_template(&discord_template_id).await?;
        let channels = revolt_discord::template::import_template(db, &mut server, template).await?;

        (server, channels)
    } else {
        Server::create(db, data, &user, true).await?
    };

    let (_, channels) = Member::create(db, &server, &user, Some(channels)).await?;

    Ok(Json(v0::CreateServerLegacyResponse {
        server: server.into(),
        channels: channels.into_iter().map(|channel| channel.into()).collect(),
    }))
}
