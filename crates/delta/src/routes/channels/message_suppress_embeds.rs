use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, PartialMessage, User,
};
use revolt_models::v0::MessageFlags;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Suppress Message Embeds
///
/// Suppress all embeds on a message and prevent future embed processing.
/// Authors can suppress embeds on their own messages, 
/// Moderators with ManageMessages can suppress on any message.
#[openapi(tag = "Messaging")]
#[post("/<target>/messages/<msg>/suppress_embeds")]
pub async fn suppress_embeds(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;
    let mut message = msg.as_message_in_channel(db, channel.id()).await?;

    // Check permissions
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;
    let is_moderator = permissions.has_channel_permission(ChannelPermission::ManageMessages);
    let is_author = message.author == user.id;

    // Ensure user can suppress embeds (either moderator or message author)
    if !is_moderator && !is_author {
        return Err(create_error!(MissingPermission {
            permission: "ManageMessages or MessageAuthor".to_string() 
        }));
    }

    let current_flags = message.flags.unwrap_or(0);

    // Check if embeds are already suppressed
    if current_flags & MessageFlags::SuppressEmbeds as u32 != 0 {
        return Err(create_error!(InvalidOperation));
    }

    // Check if message has embeds to suppress
    if message.embeds.is_none() || message.embeds.as_ref().unwrap().is_empty() {
        return Err(create_error!(InvalidOperation));
    }

    // Always set the flag and clear all embeds (same behavior for authors and moderators)
    let new_flags = current_flags | MessageFlags::SuppressEmbeds as u32;
    
    message
        .update(
            db,
            PartialMessage {
                flags: Some(new_flags),
                embeds: Some(vec![]),
                ..Default::default()
            },
            vec![],
        )
        .await?;
    Ok(EmptyResponse)
}
