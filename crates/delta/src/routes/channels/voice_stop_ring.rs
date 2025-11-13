use revolt_database::{
    util::reference::Reference,
    voice::{get_voice_state, VoiceClient},
    Channel, Database, User, AMQP,
};
use revolt_result::{create_error, Result, ToRevoltError};

use rocket::State;
use rocket_empty::EmptyResponse;

/// # Stop Ring
/// Stops ringing a specific user in a dm call.
/// You must be in the call to use this endpoint, returns NotConnected otherwise.
/// Only valid in DM/Group channels, will return NoEffect in servers.
/// Returns NotFound if the user is not in the dm/group channel
#[openapi(tag = "Voice")]
#[put("/<target>/end_ring/<target_user>")]
pub async fn stop_ring(
    db: &State<Database>,
    amqp: &State<AMQP>,
    voice: &State<VoiceClient>,
    user: User,
    target: Reference<'_>,
    target_user: Reference<'_>,
) -> Result<EmptyResponse> {
    if !voice.is_enabled() {
        return Err(create_error!(LiveKitUnavailable));
    }

    let channel = target.as_channel(db).await?;
    if channel.server().is_some() {
        return Err(create_error!(NoEffect));
    }

    if get_voice_state(channel.id(), None, &user.id)
        .await?
        .is_none()
    {
        return Err(create_error!(NotConnected));
    }

    let members = match channel {
        Channel::DirectMessage { ref recipients, .. } | Channel::Group { ref recipients, .. } => {
            recipients
        }
        _ => return Err(create_error!(NoEffect)),
    };

    if members.iter().any(|m| &target_user.id == m) {
        if let Err(e) = amqp
            .dm_call_updated(
                &user.id,
                channel.id(),
                None,
                true,
                Some(vec![target_user.id.to_string()]),
            )
            .await
            .to_internal_error()
        {
            revolt_config::capture_internal_error!(&e);
            return Err(e);
        }

        Ok(EmptyResponse)
    } else {
        Err(create_error!(NotFound))
    }
}
