use livekit_api::{access_token::TokenVerifier, webhooks::WebhookReceiver};
use livekit_protocol::TrackType;
use revolt_database::{
    events::client::EventV1,
    iso8601_timestamp::{Duration, Timestamp},
    util::reference::Reference,
    voice::{
        create_voice_state, delete_voice_state,
        get_user_moved_from_voice, get_user_moved_to_voice,
        update_voice_state_tracks, VoiceClient,
    },
    Database, AMQP,
};
use revolt_result::{Result, ToRevoltError};
use rocket::{post, State};
use rocket_empty::EmptyResponse;

use crate::guard::AuthHeader;

#[post("/<node>", data = "<body>")]
pub async fn ingress(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    _amqp: &State<AMQP>,
    node: &str,
    auth_header: AuthHeader<'_>,
    body: &str,
) -> Result<EmptyResponse> {
    log::debug!("received event: {body:?}");

    let config = revolt_config::config().await;

    let node_info = config
        .api
        .livekit
        .nodes
        .get(node)
        .to_internal_error()
        .inspect_err(|_| {
            log::error!("Unknown node {node}, make sure livekit has the correct node name set and matches `hosts.livekit` and `api.livekit.nodes` in the Revolt config.")
        })?;

    let webhook_receiver = WebhookReceiver::new(TokenVerifier::with_api_key(
        &node_info.key,
        &node_info.secret,
    ));

    let event = webhook_receiver
        .receive(body, &auth_header)
        .to_internal_error()?;

    let channel_id = event.room.as_ref().map(|r| &r.name);
    let user_id = event.participant.as_ref().map(|r| &r.identity);

    match event.event.as_str() {
        // User joined a channel
        "participant_joined" => {
            let channel_id = channel_id.to_internal_error()?;
            let user_id = user_id.to_internal_error()?;

            let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

            let joined_at = Timestamp::UNIX_EPOCH
                .checked_add(Duration::seconds(event.created_at))
                .unwrap();

            let voice_state =
                create_voice_state(channel_id, channel.server(), user_id, joined_at).await?;

            // Only publish one event when a user is moved from one channel to another.
            if let Some(moved_from) = get_user_moved_to_voice(channel_id, user_id).await? {
                EventV1::VoiceChannelMove {
                    user: user_id.to_string(),
                    from: moved_from,
                    to: channel_id.to_string(),
                    state: voice_state,
                }
                .p(channel_id.to_string())
                .await;
            } else {
                EventV1::VoiceChannelJoin {
                    id: channel_id.to_string(),
                    state: voice_state,
                }
                .p(channel_id.to_string())
                .await;
            };

            // TODO: fix `num_participants` being incorrect sometimes see (#457)
            // First user who joined - send call started system message.
            // if event.room.as_ref().unwrap().num_participants == 1 {
            //     let user = Reference::from_unchecked(user_id).as_user(db).await?;

            //     let message_id =
            //         Ulid::from_datetime(DateTime::from_timestamp_secs(event.created_at).unwrap())
            //             .to_string();

            //     let mut call_started_message = SystemMessage::CallStarted {
            //         by: user_id.to_string(),
            //         finished_at: None,
            //     }
            //     .into_message(channel.id().to_string());

            //     call_started_message.id = message_id;

            //     set_channel_call_started_system_message(channel.id(), &call_started_message.id)
            //         .await?;

            //     call_started_message
            //         .send(
            //             db,
            //             Some(amqp),
            //             v0::MessageAuthor::System {
            //                 username: &user.username,
            //                 avatar: user.avatar.as_ref().map(|file| file.id.as_ref()),
            //             },
            //             None,
            //             None,
            //             &channel,
            //             false,
            //         )
            //         .await?;

            //     let recipients = get_call_notification_recipients(&channel_id, &user_id).await?;
            //     let now = joined_at.format_short().to_string();

            //     if let Err(e) = amqp
            //         .dm_call_updated(&user.id, channel.id(), Some(&now), false, recipients)
            //         .await
            //     {
            //         revolt_config::capture_error(&e);
            //     }
            // }
        }
        // User left a channel
        "participant_left" => {
            let channel_id = channel_id.to_internal_error()?;
            let user_id = user_id.to_internal_error()?;

            let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

            delete_voice_state(channel_id, channel.server(), user_id).await?;

            // Dont send leave event when a user is moved
            if get_user_moved_from_voice(channel_id, user_id)
                .await?
                .is_none()
            {
                EventV1::VoiceChannelLeave {
                    id: channel_id.clone(),
                    user: user_id.clone(),
                }
                .p(channel_id.clone())
                .await;
            };

            // See above for why this is commented out

            // // Update CallStarted system message if everyone has left with the end time
            // let members = get_voice_channel_members(channel_id).await?;

            // if members.is_none_or(|m| m.is_empty()) {
            //     // The channel is empty so send out an "end" message for ringing
            //     if let Err(e) = amqp
            //         .dm_call_updated(user_id, channel_id, None, true, None)
            //         .await
            //     {
            //         revolt_config::capture_internal_error!(&e);
            //     }

            //     if let Some(system_message_id) =
            //         take_channel_call_started_system_message(channel_id).await?
            //     {
            //         // Could have been deleted
            //         if let Ok(mut message) = Reference::from_unchecked(&system_message_id)
            //             .as_message(db)
            //             .await
            //         {
            //             if let Some(SystemMessage::CallStarted { finished_at, .. }) =
            //                 &mut message.system
            //             {
            //                 *finished_at = Some(Timestamp::now_utc());

            //                 message
            //                     .update(
            //                         db,
            //                         PartialMessage {
            //                             system: message.system.clone(),
            //                             ..Default::default()
            //                         },
            //                         Vec::new(),
            //                     )
            //                     .await?;
            //             } else {
            //                 log::error!("Broken State: Call started message ID ({}) does not contain a CallStarted system message.", &message.id)
            //             }
            //         };
            //     };
            // }
        }
        // Audio/video track was started/stopped/unmuted/muted
        "track_published" | "track_unpublished" | "track_unmuted" | "track_muted" => {
            let channel_id = channel_id.to_internal_error()?;
            let user_id = user_id.to_internal_error()?;
            let track = event.track.as_ref().to_internal_error()?;

            let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

            let user = Reference::from_unchecked(user_id).as_user(db).await?;

            let user_limits = user.limits().await;

            // forbid any size which goes over the limit and also limit the aspect ratio to stop people from making too tall or too wide and bypassing the limit.
            // TODO: figure out how to track audio stream quality

            if event.event == "track_published" {
                let mut disconnect = false;

                if track.r#type == TrackType::Data as i32 {
                    log::debug!("User published data");
                    disconnect = true;
                };

                if track.r#type == TrackType::Video as i32 {
                    if user_limits.video_resolution[0] != 0
                        && user_limits.video_resolution[1] != 0
                        && track.width * track.height
                            > user_limits.video_resolution[0] * user_limits.video_resolution[1]
                    {
                        log::debug!("User published video with out of bounds resolution");
                        disconnect = true;
                    };

                    if user_limits.video_aspect_ratio[0] != user_limits.video_aspect_ratio[1]
                        && !(user_limits.video_aspect_ratio[0]..=user_limits.video_aspect_ratio[1])
                            .contains(&(track.width as f32 / track.height as f32))
                    {
                        log::debug!("User published video with out of bounds aspect ratio");
                        disconnect = true;
                    };
                };

                if disconnect {
                    log::debug!("Removing user {user_id} from channel {channel_id} {event:?} due to forbidden track.");

                    let _ = voice_client.remove_user(node, user_id, channel_id).await;
                    delete_voice_state(channel_id, channel.server(), user_id).await?;

                    return Ok(EmptyResponse);
                };
            };

            let partial = update_voice_state_tracks(
                channel_id,
                channel.server(),
                user_id,
                event.event == "track_published" || event.event == "track_unmuted", // to avoid duplicating this entire case twice
                track.source,
            )
            .await?;

            EventV1::UserVoiceStateUpdate {
                id: user_id.clone(),
                channel_id: channel_id.clone(),
                data: partial,
            }
            .p(channel_id.clone())
            .await;
        }
        _ => {}
    };

    Ok(EmptyResponse)
}
