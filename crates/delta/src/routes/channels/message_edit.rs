use iso8601_timestamp::Timestamp;
use revolt_database::{
    tasks,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Message, PartialMessage, User,
};
use revolt_models::v0::{self, Embed};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
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
