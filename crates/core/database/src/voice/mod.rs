use crate::{
    events::client::EventV1,
    models::{Channel, User},
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Server,
};
use iso8601_timestamp::{Duration, Timestamp};
use livekit_protocol::ParticipantPermission;
use redis_kiss::{get_connection as _get_connection, redis::Pipeline, AsyncCommands, Conn};
use revolt_config::FeaturesLimits;
use revolt_models::v0::{self, PartialUserVoiceState, UserVoiceState};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission, PermissionValue};
use revolt_result::{create_error, Result, ToRevoltError};

mod voice_client;
pub use voice_client::VoiceClient;

async fn get_connection() -> Result<Conn> {
    _get_connection().await.map_err(|_| create_error!(InternalError))
}

pub async fn raise_if_in_voice(user: &User, channel_id: &str) -> Result<()> {
    let mut conn = get_connection().await?;

    if user.bot.is_some()
    // bots can be in as many voice channels as it wants so we just check if its already connected to the one its trying to connect to
        && conn.sismember(format!("vc:{}", &user.id), channel_id)
            .await
            .to_internal_error()?
    {
        Err(create_error!(AlreadyConnected))
    } else if conn
        .scard::<_, u32>(format!("vc:{}", &user.id)) // check if the current vc set is empty
        .await
        .to_internal_error()?
        > 0
    {
        Err(create_error!(AlreadyConnected))
    } else {
        Ok(())
    }
}

pub async fn set_channel_node(channel: &str, node: &str) -> Result<()> {
    get_connection()
        .await?
        .set(format!("node:{channel}"), node)
        .await
        .to_internal_error()
}

pub async fn get_channel_node(channel: &str) -> Result<Option<String>> {
    get_connection()
        .await?
        .get(format!("node:{channel}"))
        .await
        .to_internal_error()
}

pub async fn get_user_voice_channels(user_id: &str) -> Result<Vec<String>> {
    get_connection()
        .await?
        .smembers(format!("vc:{user_id}"))
        .await
        .to_internal_error()
}

pub async fn set_user_moved_from_voice(
    old_channel: &str,
    new_channel: &str,
    user_id: &str,
) -> Result<()> {
    get_connection()
        .await?
        .set_ex(
            format!("moved_from:{user_id}:{old_channel}"),
            new_channel,
            10,
        )
        .await
        .to_internal_error()
}

pub async fn get_user_moved_from_voice(channel_id: &str, user_id: &str) -> Result<Option<String>> {
    get_connection()
        .await?
        .get_del(format!("moved_from:{user_id}:{channel_id}"))
        .await
        .to_internal_error()
}

pub async fn set_user_moved_to_voice(
    new_channel: &str,
    old_channel: &str,
    user_id: &str,
) -> Result<()> {
    get_connection()
        .await?
        .set_ex(format!("moved_to:{user_id}:{new_channel}"), old_channel, 10)
        .await
        .to_internal_error()
}

pub async fn get_user_moved_to_voice(channel_id: &str, user_id: &str) -> Result<Option<String>> {
    get_connection()
        .await?
        .get_del(format!("moved_to:{user_id}:{channel_id}"))
        .await
        .to_internal_error()
}

pub async fn is_in_voice_channel(user_id: &str, channel_id: &str) -> Result<bool> {
    get_connection()
        .await?
        .sismember(format!("vc:{user_id}"), channel_id)
        .await
        .to_internal_error()
}

pub async fn get_user_voice_channel_in_server(
    user_id: &str,
    server_id: &str,
) -> Result<Option<String>> {
    let mut conn = get_connection().await?;

    let unique_key = format!("{user_id}:{server_id}");

    conn.get(&unique_key).await.to_internal_error()
}

pub fn get_allowed_sources(
    limits: &FeaturesLimits,
    permissions: PermissionValue,
) -> Vec<&'static str> {
    let mut allowed_sources = Vec::new();

    if permissions.has(ChannelPermission::Speak as u64) {
        allowed_sources.push("microphone")
    };

    if permissions.has(ChannelPermission::Video as u64) && limits.video {
        allowed_sources.extend(["camera", "screen_share", "screen_share_audio"]);
    };

    allowed_sources
}

pub async fn create_voice_state(
    channel_id: &str,
    server_id: Option<&str>,
    user_id: &str,
    joined_at: Timestamp,
) -> Result<UserVoiceState> {
    let unique_key = format!("{}:{}", &user_id, server_id.unwrap_or(channel_id));

    let voice_state = UserVoiceState {
        joined_at,
        id: user_id.to_string(),
        is_receiving: true,
        is_publishing: false,
        screensharing: false,
        camera: false,
    };

    Pipeline::new()
        .sadd(format!("vc_members:{channel_id}"), user_id)
        .sadd(format!("vc:{user_id}"), channel_id)
        .set(&unique_key, channel_id)
        .set(
            format!("joined_at:{unique_key}"),
            joined_at
                .duration_since(Timestamp::UNIX_EPOCH)
                .whole_milliseconds() as i64,
        )
        .set(
            format!("is_publishing:{unique_key}"),
            voice_state.is_publishing,
        )
        .set(
            format!("is_receiving:{unique_key}"),
            voice_state.is_receiving,
        )
        .set(
            format!("screensharing:{unique_key}"),
            voice_state.screensharing,
        )
        .set(format!("camera:{unique_key}"), voice_state.camera)
        .query_async::<_, ()>(&mut get_connection().await?.into_inner())
        .await
        .to_internal_error()?;

    Ok(voice_state)
}

pub async fn delete_voice_state(
    channel_id: &str,
    server_id: Option<&str>,
    user_id: &str,
) -> Result<()> {
    let unique_key = format!("{}:{}", &user_id, server_id.unwrap_or(channel_id));

    Pipeline::new()
        .srem(format!("vc_members:{channel_id}"), user_id)
        .srem(format!("vc:{user_id}"), channel_id)
        .del(&[
            format!("joined_at:{unique_key}"),
            format!("is_publishing:{unique_key}"),
            format!("is_receiving:{unique_key}"),
            format!("screensharing:{unique_key}"),
            format!("camera:{unique_key}"),
            unique_key.clone(),
        ])
        .query_async(&mut get_connection().await?.into_inner())
        .await
        .to_internal_error()
}

pub async fn delete_channel_voice_state(
    channel_id: &str,
    server_id: Option<&str>,
    user_ids: &[String],
) -> Result<()> {
    let parent_id = server_id.unwrap_or(channel_id);

    let mut pipeline = Pipeline::new();
    pipeline.del(format!("vc_members:{channel_id}"));

    for user_id in user_ids {
        let unique_key = format!("{user_id}:{parent_id}");

        pipeline.srem(format!("vc:{user_id}"), channel_id).del(&[
            format!("joined_at:{unique_key}"),
            format!("is_publishing:{unique_key}"),
            format!("is_receiving:{unique_key}"),
            format!("screensharing:{unique_key}"),
            format!("camera:{unique_key}"),
            unique_key.clone(),
        ]);
    }

    pipeline
        .query_async(&mut get_connection().await?.into_inner())
        .await
        .to_internal_error()
}

pub async fn update_voice_state_tracks(
    channel_id: &str,
    server_id: Option<&str>,
    user_id: &str,
    added: bool,
    track: i32,
) -> Result<PartialUserVoiceState> {
    let partial = match track {
        /* TrackSource::Unknown */ 0 => PartialUserVoiceState::default(),
        /* TrackSource::Camera */
        1 => PartialUserVoiceState {
            camera: Some(added),
            ..Default::default()
        },
        /* TrackSource::Microphone */
        2 => PartialUserVoiceState {
            is_publishing: Some(added),
            ..Default::default()
        },
        /* TrackSource::ScreenShare | TrackSource::ScreenShareAudio */
        3 | 4 => PartialUserVoiceState {
            screensharing: Some(added),
            ..Default::default()
        },
        _ => unreachable!(),
    };

    update_voice_state(channel_id, server_id, user_id, &partial).await?;

    Ok(partial)
}

pub async fn update_voice_state(
    channel_id: &str,
    server_id: Option<&str>,
    user_id: &str,
    partial: &PartialUserVoiceState,
) -> Result<()> {
    let unique_key = format!("{}:{}", &user_id, server_id.unwrap_or(channel_id));

    let mut pipeline = Pipeline::new();

    if let Some(camera) = &partial.camera {
        pipeline.set(format!("camera:{unique_key}"), camera);
    };

    if let Some(is_publishing) = &partial.is_publishing {
        pipeline.set(format!("is_publishing:{unique_key}"), is_publishing);
    }

    if let Some(is_receiving) = &partial.is_receiving {
        pipeline.set(format!("is_receiving:{unique_key}"), is_receiving);
    }

    if let Some(screensharing) = &partial.screensharing {
        pipeline.set(format!("screensharing:{unique_key}"), screensharing);
    }

    pipeline
        .query_async(&mut get_connection().await?.into_inner())
        .await
        .to_internal_error()
}

pub async fn get_voice_channel_members(channel_id: &str) -> Result<Option<Vec<String>>> {
    get_connection()
        .await?
        .smembers::<_, Option<Vec<String>>>(format!("vc_members:{channel_id}"))
        .await
        .to_internal_error()
        .map(|opt| opt.and_then(|v| if v.is_empty() { None } else { Some(v) }))
}

pub async fn get_voice_state(
    channel_id: &str,
    server_id: Option<&str>,
    user_id: &str,
) -> Result<Option<UserVoiceState>> {
    let unique_key = format!("{}:{}", user_id, server_id.unwrap_or(channel_id));

    let (joined_at, is_publishing, is_receiving, screensharing, camera) = get_connection()
        .await?
        .mget(&[
            format!("joined_at:{unique_key}"),
            format!("is_publishing:{unique_key}"),
            format!("is_receiving:{unique_key}"),
            format!("screensharing:{unique_key}"),
            format!("camera:{unique_key}"),
        ])
        .await
        .to_internal_error()?;

    match (
        joined_at,
        is_publishing,
        is_receiving,
        screensharing,
        camera,
    ) {
        (
            Some(joined_at),
            Some(is_publishing),
            Some(is_receiving),
            Some(screensharing),
            Some(camera),
        ) => Ok(Some(v0::UserVoiceState {
            joined_at: Timestamp::UNIX_EPOCH
                .checked_add(Duration::milliseconds(joined_at))
                .unwrap(),
            id: user_id.to_string(),
            is_receiving,
            is_publishing,
            screensharing,
            camera,
        })),
        _ => Ok(None),
    }
}

pub async fn get_channel_voice_state(channel: &Channel) -> Result<Option<v0::ChannelVoiceState>> {
    let members = get_voice_channel_members(channel.id()).await?;

    let server = channel.server();

    if let Some(members) = members {
        let mut participants = Vec::with_capacity(members.len());

        for user_id in members {
            if let Some(voice_state) = get_voice_state(channel.id(), server, &user_id).await? {
                participants.push(voice_state);
            } else {
                log::info!("Voice state not found but member in voice channel members, removing.");

                delete_voice_state(channel.id(), server, &user_id).await?;
            }
        }

        // In case a user voice state failed to be fetched, the vec's capacity will be larger than the length, shrink it
        participants.shrink_to_fit();

        Ok(Some(v0::ChannelVoiceState {
            id: channel.id().to_string(),
            participants,
        }))
    } else {
        Ok(None)
    }
}

pub async fn move_user(user: &str, from: &str, to: &str) -> Result<()> {
    get_connection()
        .await?
        .smove(
            format!("vc-members-{from}"),
            format!("vc-members-{to}"),
            user,
        )
        .await
        .to_internal_error()
}

pub async fn sync_voice_permissions(
    db: &Database,
    voice_client: &VoiceClient,
    channel: &Channel,
    server: Option<&Server>,
    role_id: Option<&str>,
) -> Result<()> {
    let Some(node) = get_channel_node(channel.id()).await? else {
        return Ok(());
    };

    for user_id in get_voice_channel_members(channel.id())
        .await?
        .iter()
        .flatten()
    {
        let user = Reference::from_unchecked(user_id).as_user(db).await?;

        sync_user_voice_permissions(db, voice_client, &node, &user, channel, server, role_id)
            .await?;
    }

    Ok(())
}

pub async fn sync_user_voice_permissions(
    db: &Database,
    voice_client: &VoiceClient,
    node: &str,
    user: &User,
    channel: &Channel,
    server: Option<&Server>,
    role_id: Option<&str>,
) -> Result<()> {
    let channel_id = channel.id();
    let server_id = server.as_ref().map(|s| s.id.as_str());

    let member = match server_id {
        Some(server_id) => Some(
            Reference::from_unchecked(&user.id)
                .as_member(db, server_id)
                .await?,
        ),
        None => None,
    };

    if role_id.is_none_or(|role_id| {
        member
            .as_ref()
            .is_none_or(|member| member.roles.iter().any(|r| r == role_id))
    }) {
        let Some(voice_state) = get_voice_state(channel_id, server_id, &user.id).await? else {
            return Ok(());
        };

        let mut query = DatabasePermissionQuery::new(db, user)
            .channel(channel)
            .user(user);

        if let (Some(server), Some(member)) = (server, member.as_ref()) {
            query = query.member(member).server(server)
        }

        let permissions = calculate_channel_permissions(&mut query).await;
        let limits = user.limits().await;

        let mut update_event = PartialUserVoiceState {
            id: Some(user.id.clone()),
            ..Default::default()
        };

        let before = update_event.clone();

        let can_video =
            limits.video && permissions.has_channel_permission(ChannelPermission::Video);
        let can_speak = permissions.has_channel_permission(ChannelPermission::Speak);
        let can_listen = permissions.has_channel_permission(ChannelPermission::Listen);

        update_event.camera = voice_state.camera.then_some(can_video);
        update_event.screensharing = voice_state.screensharing.then_some(can_video);
        update_event.is_publishing = voice_state.is_publishing.then_some(can_speak);

        update_voice_state(channel_id, server_id, &user.id, &update_event).await?;

        voice_client
            .update_permissions(
                node,
                user,
                channel_id,
                ParticipantPermission {
                    can_subscribe: can_listen,
                    can_publish: can_speak,
                    can_publish_data: can_speak,
                    ..Default::default()
                },
            )
            .await?;

        if update_event != before {
            EventV1::UserVoiceStateUpdate {
                id: user.id.clone(),
                channel_id: channel_id.to_string(),
                data: update_event,
            }
            .p(channel_id.to_string())
            .await;
        };
    };

    Ok(())
}

pub async fn set_channel_call_started_system_message(
    channel_id: &str,
    message_id: &str,
) -> Result<()> {
    get_connection()
        .await?
        .set(format!("call_started_message:{channel_id}"), message_id)
        .await
        .to_internal_error()
}

pub async fn take_channel_call_started_system_message(channel_id: &str) -> Result<Option<String>> {
    get_connection()
        .await?
        .get_del(format!("call_started_message:{channel_id}"))
        .await
        .to_internal_error()
}

pub async fn set_call_notification_recipients(
    channel_id: &str,
    user_id: &str,
    recipients: &[String],
) -> Result<()> {
    get_connection()
        .await?
        .set_ex(
            format!("call_notification_recipients:{channel_id}-{user_id}"),
            recipients,
            10,
        )
        .await
        .to_internal_error()
}

pub async fn get_call_notification_recipients(
    channel_id: &str,
    user_id: &str,
) -> Result<Option<Vec<String>>> {
    get_connection()
        .await?
        .get_del(format!(
            "call_notification_recipients:{channel_id}-{user_id}"
        ))
        .await
        .to_internal_error()
}

pub async fn remove_user_from_voice_channels(db: &Database, voice_client: &VoiceClient, user_id: &str) -> Result<()> {
    for channel_id in get_user_voice_channels(user_id).await? {
        remove_user_from_voice_channel(db, voice_client, &channel_id, user_id).await?;
    };

    Ok(())
}

pub async fn remove_user_from_voice_channel(db: &Database, voice_client: &VoiceClient, channel_id: &str, user_id: &str) -> Result<()> {
    if let Some(node) = get_channel_node(channel_id).await? {
        let _ = voice_client.remove_user(&node, user_id, channel_id).await;
    }

    let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

    delete_voice_state(channel_id, channel.server(), user_id).await?;

    Ok(())
}

pub async fn delete_voice_channel(voice_client: &VoiceClient, channel_id: &str, server_id: Option<&str>) -> Result<()> {
    if let Some(users) = get_voice_channel_members(channel_id).await? {
        let node = get_channel_node(channel_id).await?.unwrap();

        voice_client.delete_room(&node, channel_id).await?;

        delete_channel_voice_state(channel_id, server_id, &users).await?;
    };

    Ok(())
}