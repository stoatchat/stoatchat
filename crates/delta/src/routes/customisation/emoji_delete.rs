use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Database, EmojiParent, User,
};
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::Result;

use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::audit_log_reason::AuditLogReason;

/// # Delete Emoji
///
/// Delete an emoji by its id.
#[openapi(tag = "Emojis")]
#[delete("/emoji/<emoji_id>")]
pub async fn delete_emoji(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    emoji_id: Reference<'_>,
) -> Result<EmptyResponse> {
    // Fetch the emoji
    let emoji = emoji_id.as_emoji(db).await?;

    // If we uploaded the emoji, then we have permission to delete it
    if emoji.creator_id != user.id {
        // Otherwise, validate we have permission to delete from parent
        match &emoji.parent {
            EmojiParent::Server { id } => {
                let server = db.fetch_server(id.as_str()).await?;

                // Check for permission
                let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
                calculate_server_permissions(&mut query)
                    .await
                    .throw_if_lacking_channel_permission(ChannelPermission::ManageCustomisation)?;
            }
            EmojiParent::Detached => return Ok(EmptyResponse),
        };
    }

    // Delete the emoji
    emoji.delete(db).await?;

    if let EmojiParent::Server { id: server_id } = emoji.parent {
        AuditLogEntryAction::EmojiDelete {
            emoji: emoji.id,
            name: emoji.name,
        }
        .insert(db, server_id, reason.0, user.id)
        .await;
    };

    Ok(EmptyResponse)
}
