use iso8601_timestamp::Timestamp;
use revolt_config::config;
use revolt_database::{
    tasks::process_embeds::queue, util::reference::Reference, Database, Message, PartialMessage,
};
use revolt_models::v0::{self, DataEditMessage, Embed};
use revolt_models::validator::Validate;
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Edits a webhook message
///
/// Edits a message sent by a webhook
#[openapi(tag = "Webhooks")]
#[patch("/<webhook_id>/<token>/<message_id>", data = "<data>")]
pub async fn webhook_edit_message(
    db: &State<Database>,
    webhook_id: Reference<'_>,
    token: String,
    message_id: Reference<'_>,
    data: Json<DataEditMessage>,
) -> Result<Json<v0::Message>> {
    let edit = data.into_inner();
    edit.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    Message::validate_sum(
        &edit.content,
        edit.embeds.as_deref().unwrap_or_default(),
        config().await.features.limits.default.message_length,
    )?;

    let webhook = webhook_id.as_webhook(db).await?;
    webhook.assert_token(&token)?;

    let mut message = message_id.as_message(db).await?;
    if message.author != webhook.id {
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
    let mut new_embeds = vec![];
    if let Some(embeds) = &message.embeds {
        for embed in embeds {
            if let Embed::Text(embed) = embed {
                new_embeds.push(Embed::Text(embed.clone()))
            }
        }
    }

    // 3. Replace if we are given new embeds
    if let Some(embeds) = edit.embeds {
        new_embeds.clear();

        for embed in embeds {
            new_embeds.push(message.create_embed(db, embed).await?);
        }
    }

    partial.embeds = Some(new_embeds);

    message.update(db, partial, vec![]).await?;

    // Queue up a task for processing embeds if the we have sufficient permissions
    if let Some(content) = edit.content {
        queue(message.channel.to_string(), message.id.to_string(), content).await;
    }

    Ok(Json(message.into_model(None, None)))
}
