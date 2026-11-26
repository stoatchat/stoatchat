use iso8601_timestamp::Timestamp;
use revolt_database::{
    Database, Message, PartialMessage, User, tasks,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
};
use revolt_models::v0::{self, Embed};
use revolt_permissions::{ChannelPermission, calculate_channel_permissions};
use revolt_result::{Result, create_error};
use rocket::{State, serde::json::Json};
use validator::Validate;

/// # Edit Message
///
/// Edits a message that you've previously sent.
#[openapi(tag = "Messaging")]
#[patch("/<target>/messages/<msg>", data = "<edit>")]
pub async fn edit(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
    edit: Json<v0::DataEditMessage>,
) -> Result<Json<v0::Message>> {
    let edit = edit.into_inner();
    edit.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    Message::validate_sum(
        &edit.content,
        edit.embeds.as_deref().unwrap_or_default(),
        user.limits().await.message_length,
    )?;

    // Ensure we have permissions to send a message
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::SendMessage)?;

    // Validate external emoji usage
    let emoji_ids: Vec<String> = Message::collect_emojis(
        edit.content.as_deref(),
        edit.embeds.as_deref().unwrap_or_default(),
    )
    .into_iter()
    .collect();
    Message::validate_external_emojis(db, &channel, &permissions, &emoji_ids).await?;

    let mut message = msg.as_message_in_channel(db, channel.id()).await?;
    if message.author != user.id {
        return Err(create_error!(CannotEditMessage));
    }

    message.edited = Some(Timestamp::now_utc());
    let mut partial = PartialMessage {
        edited: message.edited,
        ..Default::default()
    };

    // 1. Handle content update
    if let Some(content) = &edit.content {
        partial.content = Some(content.clone());
    }

    // 2. Clear any auto generated embeds
    let mut new_embeds: Vec<Embed> = vec![];
    if let Some(embeds) = &message.embeds {
        for embed in embeds {
            if let Embed::Text(embed) = embed {
                new_embeds.push(Embed::Text(embed.clone()))
            }
        }
    }

    // 3. Replace if we are given new embeds
    if let Some(embeds) = edit.embeds {
        // Ensure we have permissions to send embeds
        permissions.throw_if_lacking_channel_permission(ChannelPermission::SendEmbeds)?;

        new_embeds.clear();

        for embed in embeds {
            new_embeds.push(message.create_embed(db, embed).await?);
        }
    }

    partial.embeds = Some(new_embeds);

    message.update(db, partial, vec![]).await?;

    // Queue up a task for processing embeds if the we have sufficient permissions
    if permissions.has_channel_permission(ChannelPermission::SendEmbeds) {
        if let Some(content) = edit.content {
            tasks::process_embeds::queue(
                message.channel.to_string(),
                message.id.to_string(),
                content,
            )
            .await;
        }
    }

    Ok(Json(message.into_model(None, None)))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Channel, Emoji, Member, PartialChannel, Session};
    use revolt_models::v0;
    use revolt_permissions::{ChannelPermission, OverrideField};
    use rocket::http::{ContentType, Header, Status};
    use serde_json::Value;

    async fn emoji_server_setup(harness: &TestHarness) -> (Session, Channel, Channel, Emoji) {
        let (_, _, owner) = harness.new_user().await;
        let (server, channels) = harness.new_server(&owner).await;
        let (other_server, _) = harness.new_server(&owner).await;
        let emoji = harness.new_emoji(&other_server, &owner).await;

        let (_, session, user) = harness.new_user().await;
        Member::create(&harness.db, &server, &user, Some(channels))
            .await
            .expect("Failed to create member");

        let allowed = harness.new_channel(&server).await;
        let mut denied = harness.new_channel(&server).await;
        denied
            .update(
                &harness.db,
                PartialChannel {
                    default_permissions: Some(OverrideField {
                        a: 0,
                        d: ChannelPermission::UseExternalEmojis as i64,
                    }),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to deny UseExternalEmojis");

        (session, allowed, denied, emoji)
    }

    async fn send_plain(
        harness: &TestHarness,
        session: &Session,
        channel: &Channel,
    ) -> v0::Message {
        let response = harness
            .client
            .post(format!("/channels/{}/messages", channel.id()))
            .header(ContentType::JSON)
            .header(Header::new("x-session-token", session.token.to_string()))
            .body(json!({ "content": "hello" }).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        response.into_json().await.expect("`Message`")
    }

    async fn edit(
        harness: &TestHarness,
        session: &Session,
        channel: &Channel,
        body: Value,
    ) -> Status {
        let message = send_plain(harness, session, channel).await;

        harness
            .client
            .patch(format!(
                "/channels/{}/messages/{}",
                channel.id(),
                message.id
            ))
            .header(ContentType::JSON)
            .header(Header::new("x-session-token", session.token.to_string()))
            .body(body.to_string())
            .dispatch()
            .await
            .status()
    }

    #[rocket::async_test]
    async fn edit_content_external_emoji() {
        let harness = TestHarness::new().await;
        let (session, allowed, denied, emoji) = emoji_server_setup(&harness).await;
        let body = json!({ "content": format!(":{}:", emoji.id) });

        assert_eq!(
            edit(&harness, &session, &allowed, body.clone()).await,
            Status::Ok
        );
        assert_eq!(
            edit(&harness, &session, &denied, body).await,
            Status::Forbidden,
            "External emoji was edited into content without UseExternalEmojis"
        );
    }

    #[rocket::async_test]
    async fn edit_embed_description_external_emoji() {
        let harness = TestHarness::new().await;
        let (session, allowed, denied, emoji) = emoji_server_setup(&harness).await;
        let body = json!({ "embeds": [{ "description": format!(":{}:", emoji.id) }] });

        assert_eq!(
            edit(&harness, &session, &allowed, body.clone()).await,
            Status::Ok
        );
        assert_eq!(
            edit(&harness, &session, &denied, body).await,
            Status::Forbidden,
            "External emoji was edited into embed description without UseExternalEmojis"
        );
    }

    #[rocket::async_test]
    async fn edit_embed_title_external_emoji() {
        let harness = TestHarness::new().await;
        let (session, allowed, denied, emoji) = emoji_server_setup(&harness).await;
        let body = json!({ "embeds": [{ "title": format!(":{}:", emoji.id) }] });

        assert_eq!(
            edit(&harness, &session, &allowed, body.clone()).await,
            Status::Ok
        );
        assert_eq!(
            edit(&harness, &session, &denied, body).await,
            Status::Forbidden,
            "External emoji was edited into embed title without UseExternalEmojis"
        );
    }
}
