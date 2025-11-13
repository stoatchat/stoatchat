use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{get_user_voice_channel_in_server, remove_user_from_voice_channel, VoiceClient},
    Database, RemovalIntention, ServerBan, User,
};
use revolt_models::v0;

use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Ban User
///
/// Ban a user by their id.
#[openapi(tag = "Server Members")]
#[put("/<server>/bans/<target>", data = "<data>")]
pub async fn ban(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    server: Reference<'_>,
    target: Reference<'_>,
    data: Json<v0::DataBanCreate>,
) -> Result<Json<v0::ServerBan>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let server = server.as_server(db).await?;

    if target.id == user.id {
        return Err(create_error!(CannotRemoveYourself));
    }

    if target.id == server.owner {
        return Err(create_error!(InvalidOperation));
    }

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::BanMembers)?;

    // If member exists, check privileges against them
    if let Ok(member) = target.as_member(db, &server.id).await {
        if member.get_ranking(query.server_ref().as_ref().unwrap())
            <= query.get_member_rank().unwrap_or(i64::MIN)
        {
            return Err(create_error!(NotElevated));
        }

        member
            .remove(db, &server, RemovalIntention::Ban, false)
            .await?;

        // If the member is in a voice channel while banned kick them from the voice channel
        if let Some(channel_id) = get_user_voice_channel_in_server(&target.id, &server.id).await? {
            remove_user_from_voice_channel(db, voice_client, &channel_id, &target.id).await?;
        }
    }

    ServerBan::create(db, &server, target.id, data.reason)
        .await
        .map(Into::into)
        .map(Json)
}
