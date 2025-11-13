use revolt_config::config;
use revolt_database::{
    util::{permissions::perms, reference::Reference},
    voice::{
        delete_voice_state, get_channel_node, get_user_voice_channels, get_voice_channel_members,
        raise_if_in_voice, set_call_notification_recipients, VoiceClient,
    },
    Database, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::{serde::json::Json, State};

/// # Join Call
///
/// Asks the voice server for a token to join the call.
#[openapi(tag = "Voice")]
#[post("/<target>/join_call", data = "<data>")]
pub async fn call(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::DataJoinCall>,
) -> Result<Json<v0::CreateVoiceUserResponse>> {
    if !voice_client.is_enabled() {
        return Err(create_error!(LiveKitUnavailable));
    }

    let v0::DataJoinCall {
        node,
        force_disconnect,
        recipients,
    } = data.into_inner();

    if user.bot.is_some() && force_disconnect == Some(true) {
        return Err(create_error!(IsBot));
    }

    let channel = target.as_channel(db).await?;

    let Some(voice_info) = channel.voice() else {
        return Err(create_error!(NotAVoiceChannel));
    };

    let mut permissions = perms(db, &user).channel(&channel);

    let current_permissions = calculate_channel_permissions(&mut permissions).await;
    current_permissions.throw_if_lacking_channel_permission(ChannelPermission::Connect)?;

    if get_voice_channel_members(channel.id())
        .await?
        .zip(voice_info.max_users)
        .is_some_and(|(ms, max_users)| ms.len() >= max_users)
        && !current_permissions.has(ChannelPermission::ManageChannel as u64)
    {
        return Err(create_error!(CannotJoinCall));
    }

    let existing_node = get_channel_node(channel.id()).await?;

    let node = existing_node
        .or(node)
        .ok_or_else(|| create_error!(UnknownNode))?;

    let config = config().await;

    let node_host = config
        .hosts
        .livekit
        .get(&node)
        .ok_or_else(|| create_error!(UnknownNode))?
        .clone();

    if force_disconnect == Some(true) {
        // Finds and disconnects any existing voice connections by the user,
        // should only ever loop once but just to cover our backs.

        for channel_id in get_user_voice_channels(&user.id).await? {
            if let Some(node) = get_channel_node(&channel_id).await? {
                // if this errors its just a mismatching state - ignore and proceed to still delete our state
                let _ = voice_client.remove_user(&node, &user.id, &channel_id).await;
            };

            let channel = Reference::from_unchecked(&channel_id)
                .as_channel(db)
                .await?;

            delete_voice_state(&channel_id, channel.server(), &user.id).await?;
        }
    } else {
        raise_if_in_voice(&user, channel.id()).await?;
    }

    let token = voice_client
        .create_token(&node, db, &user, current_permissions, &channel)
        .await?;

    let room = voice_client.create_room(&node, &channel).await?;

    log::debug!("Created room {}", room.name);

    if let Some(recipients) = recipients {
        if room.num_participants == 0 && !recipients.is_empty() {
            set_call_notification_recipients(channel.id(), &user.id, &recipients).await?;
        }
    }

    Ok(Json(v0::CreateVoiceUserResponse {
        token,
        url: node_host.clone(),
    }))
}
