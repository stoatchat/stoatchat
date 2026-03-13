use std::time::{SystemTime, Duration};
use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{
        get_user_voice_channel_in_server, remove_user_from_voice_channel, UserVoiceChannel,
        VoiceClient,
    },
    Database, RemovalIntention, ServerBan, User,
};
use revolt_models::v0;

use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use ulid::Ulid;
use validator::Validate;
use revolt_database::events::client::EventV1;

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
        if let Some(channel_id) = get_user_voice_channel_in_server(target.id, &server.id).await? {
            remove_user_from_voice_channel(
                voice_client,
                &UserVoiceChannel {
                    id: channel_id,
                    server_id: Some(server.id.clone()),
                },
                target.id,
            )
            .await?;
        }
    }
    // We do this outside the member check so we can sweep hit-and-run spammers who already left.
    if let Some(seconds) = data.delete_message_seconds {
        if seconds > 0 {
            let threshold_time = SystemTime::now() - Duration::from_secs(seconds as u64);

            let deleted_groups = db.delete_messages_by_author_since(
                &server.channels,
                target.id,
                threshold_time
            ).await?;

            for (channel_id, message_ids) in deleted_groups {
                if !message_ids.is_empty() {
                    EventV1::BulkMessageDelete {
                        channel: channel_id.clone(),
                        ids: message_ids,
                    }
                        .p(channel_id)
                        .await;
                }
            }
        }
    }
    ServerBan::create(db, &server, target.id, data.reason)
        .await
        .map(Into::into)
        .map(Json)
}
