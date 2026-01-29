use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Database, User,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::audit_log_reason::AuditLogReason;

/// # Delete Message
///
/// Delete a message you've sent or one you have permission to delete.
#[openapi(tag = "Messaging")]
#[delete("/<target>/messages/<msg>", rank = 2)]
pub async fn delete(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let message = msg.as_message_in_channel(db, target.id).await?;

    let channel = if message.author != user.id {
        let channel = target.as_channel(db).await?;
        let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
        calculate_channel_permissions(&mut query)
            .await
            .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;

        Some(channel)
    } else {
        None
    };

    message.delete(db).await?;

    if let Some(server) = channel.and_then(|c| c.server().map(|s| s.to_string())) {
        AuditLogEntryAction::MessageDelete {
            author: message.author.clone(),
            channel: message.channel.clone(),
        }
        .insert(db, server.to_string(), reason.0, user.id.clone())
        .await;
    };

    Ok(EmptyResponse)
}
