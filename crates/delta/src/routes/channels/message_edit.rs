use iso8601_timestamp::Timestamp;
use revolt_database::{
    tasks,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Message, PartialMessage, User,
};
use revolt_models::v0::{self, Embed, MessageFlags};
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

    // Check if embeds are suppressed
    let current_flags = message.flags.unwrap_or(0);
    let embeds_suppressed = current_flags & MessageFlags::SuppressEmbeds as u32 != 0;
    

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
            new_embeds.push(
                message.create_embed(db, embed).await?
            );
        }
    }

    // 4. Handle attachment removal
    if let Some(remove_attachments) = &edit.remove_attachments {
        // Validate that all attachment IDs exist in the message
        if let Some(ref attachments) = message.attachments {
            for attachment_id in remove_attachments {
                if !attachments.iter().any(|file| &file.id == attachment_id) {
                    return Err(create_error!(NotFound));
                }
            }

            // Remove specified attachments
            let mut updated_attachments = attachments.clone();
            let removed_files: Vec<_> = attachments
                .iter()
                .filter(|file| remove_attachments.contains(&file.id))
                .cloned()
                .collect();

            updated_attachments.retain(|file| !remove_attachments.contains(&file.id));
            partial.attachments = Some(updated_attachments);

            // Mark removed files for cleanup
            for file in removed_files {
                db.mark_attachment_as_deleted(&file.id).await?;
            }
        } else if !remove_attachments.is_empty() {
            return Err(create_error!(InvalidOperation));
        }
    }

    partial.embeds = Some(new_embeds);

    message.update(db, partial, vec![]).await?;

    // Queue up a task for processing embeds if sufficient permissions and embeds aren't suppressed
    if permissions.has_channel_permission(ChannelPermission::SendEmbeds) && !embeds_suppressed {
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
