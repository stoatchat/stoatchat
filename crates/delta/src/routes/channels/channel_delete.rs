use revolt_database::{
    AMQP, AuditLogEntryAction, Channel, Database, PartialChannel, User,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{
        UserVoiceChannel, VoiceClient, delete_voice_channel, is_in_voice_channel,
        remove_user_from_voice_channel,
    },
};
use revolt_models::v0;
use revolt_permissions::{ChannelPermission, calculate_channel_permissions};
use revolt_result::{Result, create_error};
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::audit_log_reason::AuditLogReason;

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
    reason: AuditLogReason,
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
            if channel.is_system_dm() {
                return Err(create_error!(InvalidOperation));
            }

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

            let user_voice_channel = UserVoiceChannel::from_channel(&channel);

            if is_in_voice_channel(&user.id, &user_voice_channel).await? {
                remove_user_from_voice_channel(voice_client, &user_voice_channel, &user.id).await?;
            };
        }
        Channel::TextChannel { name, server, .. } => {
            permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;
            channel.delete(db).await?;

            AuditLogEntryAction::ChannelDelete {
                channel: channel.id().to_string(),
                name: name.clone(),
            }
            .insert(db, server.clone(), reason, user.id, None)
            .await;

            delete_voice_channel(voice_client, &UserVoiceChannel::from_channel(&channel)).await?;
        }
    };

    Ok(EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::util::test::PubSubTestHelper;
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Channel, events::client::EventV1};
    use revolt_models::v0::{DataCreateGroup, SYSTEM_USER_ID};
    use rocket::http::{ContentType, Header, Status};
    use serde_json::json;

    #[rocket::async_test]
    async fn success_delete_group() {
        let harness = TestHarness::new().await;
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

        let mut pubsub = PubSubTestHelper::new(group.id()).await;

        let response = harness
            .client
            .delete(format!("/channels/{}", group.id()))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        drop(response);

        pubsub
            .wait_for_event(|event| match event {
                EventV1::ChannelDelete { id, .. } => id == group.id(),
                _ => false,
            })
            .await;
    }

    #[rocket::async_test]
    async fn fail_interact_with_system_dm() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let message = user
            .send_system_dm(&harness.db, "Account notice".to_string())
            .await
            .expect("`Message`");

        let token = || Header::new("x-session-token", session.token.to_string());

        // Can read the channel
        let response = harness
            .client
            .get(format!("/channels/{}/messages", message.channel))
            .header(token())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        drop(response);

        // Cannot send, react, pin or close
        let response = harness
            .client
            .post(format!("/channels/{}/messages", message.channel))
            .header(ContentType::JSON)
            .header(token())
            .body(json!({ "content": "hi" }).to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        drop(response);

        let response = harness
            .client
            .put(format!(
                "/channels/{}/messages/{}/reactions/%F0%9F%91%8D",
                message.channel, message.id
            ))
            .header(token())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        drop(response);

        let response = harness
            .client
            .post(format!(
                "/channels/{}/messages/{}/pin",
                message.channel, message.id
            ))
            .header(token())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        drop(response);

        let response = harness
            .client
            .delete(format!("/channels/{}", message.channel))
            .header(token())
            .dispatch()
            .await;
        assert_ne!(response.status(), Status::NoContent);
        drop(response);

        // Cannot block or friend the system user
        let response = harness
            .client
            .put(format!("/users/{SYSTEM_USER_ID}/block"))
            .header(token())
            .dispatch()
            .await;
        assert_ne!(response.status(), Status::Ok);
        drop(response);

        let response = harness
            .client
            .put(format!("/users/{SYSTEM_USER_ID}/friend"))
            .header(token())
            .dispatch()
            .await;
        assert_ne!(response.status(), Status::Ok);
        drop(response);

        let channel = harness
            .db
            .fetch_channel(&message.channel)
            .await
            .expect("`Channel`");
        assert!(matches!(
            channel,
            Channel::DirectMessage { active: true, .. }
        ));
    }

    // TEST: member leaves group (no delete)
    // TEST: no effect with saved messages
    // TEST: DM set to inactive

    #[rocket::async_test]
    async fn success_delete_channel() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (_, channels) = harness.new_server(&user).await;
        let mut pubsub = PubSubTestHelper::new(channels[0].id()).await;
        let response = TestHarness::with_session(
            session,
            harness
                .client
                .delete(format!("/channels/{}", channels[0].id())),
        )
        .await;
        assert_eq!(response.status(), Status::NoContent);
        drop(response);
        pubsub
            .wait_for_event(|event| match event {
                EventV1::ChannelDelete { id, .. } => id == channels[0].id(),
                _ => false,
            })
            .await;
    }
}
