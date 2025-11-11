use revolt_database::{
    AMQP, Channel, Database, PartialChannel, User, util::{permissions::DatabasePermissionQuery, reference::Reference}, voice::{
        VoiceClient, delete_voice_channel, is_in_voice_channel, remove_user_from_voice_channel
    }
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Close Channel
///
/// Deletes a server channel, leaves a group or closes a group.
#[openapi(tag = "Channel Information")]
#[delete("/<target>?<options..>")]
pub async fn delete(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    amqp: &State<AMQP>,
    user: User,
    target: Reference<'_>,
    options: v0::OptionsChannelDelete,
) -> Result<EmptyResponse> {
    let mut channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ViewChannel)?;

    #[allow(deprecated)]
    match &channel {
        Channel::SavedMessages { .. } => Err(create_error!(NoEffect))?,
        Channel::DirectMessage { .. } => {
            channel
                .update(
                    db,
                    PartialChannel {
                        active: Some(false),
                        ..Default::default()
                    },
                    vec![],
                )
                .await?
        }
        Channel::Group { .. } => {
            channel
                .remove_user_from_group(
                    db,
                    amqp,
                    &user,
                    None,
                    options.leave_silently.unwrap_or_default(),
                )
                .await?;

            if is_in_voice_channel(&user.id, channel.id()).await? {
                remove_user_from_voice_channel(db, voice_client, channel.id(), &user.id).await?;
            };
        }
        Channel::TextChannel { .. } => {
            permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;
            channel.delete(db).await?;

            delete_voice_channel(voice_client, channel.id(), channel.server()).await?;
        }
    };

    Ok(EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{events::client::EventV1, Channel};
    use revolt_models::v0::DataCreateGroup;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn success_delete_group() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let group = Channel::create_group(
            &harness.db,
            DataCreateGroup {
                ..Default::default()
            },
            user.id.clone(),
        )
        .await
        .expect("`Channel`");

        let response = harness
            .client
            .delete(format!("/channels/{}", group.id()))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        drop(response);

        harness
            .wait_for_event(group.id(), |event| match event {
                EventV1::ChannelDelete { id, .. } => id == group.id(),
                _ => false,
            })
            .await;
    }

    // TEST: member leaves group (no delete)
    // TEST: no effect with saved messages
    // TEST: DM set to inactive

    #[rocket::async_test]
    async fn success_delete_channel() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (_, channels) = harness.new_server(&user).await;
        let response = TestHarness::with_session(
            session,
            harness
                .client
                .delete(format!("/channels/{}", channels[0].id())),
        )
        .await;
        assert_eq!(response.status(), Status::NoContent);
        drop(response);
        harness
            .wait_for_event(channels[0].id(), |event| match event {
                EventV1::ChannelDelete { id, .. } => id == channels[0].id(),
                _ => false,
            })
            .await;
    }
}
