use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Database, Invite, User,
};
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::audit_log_reason::AuditLogReason;

/// # Delete Invite
///
/// Delete an invite by its id.
#[openapi(tag = "Invites")]
#[delete("/<target>")]
pub async fn delete(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
) -> Result<EmptyResponse> {
    let invite = target.as_invite(db).await?;

    if user.id == invite.creator() {
        db.delete_invite(invite.code()).await?;
    } else {
        match invite {
            Invite::Server {
                code,
                server,
                channel,
                ..
            } => {
                let server = db.fetch_server(&server).await?;
                let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
                calculate_server_permissions(&mut query)
                    .await
                    .throw_if_lacking_channel_permission(ChannelPermission::ManageServer)?;

                db.delete_invite(&code).await?;

                AuditLogEntryAction::InviteDelete {
                    invite: code,
                    channel: channel,
                }
                .insert(db, server.id, reason.0, user.id)
                .await;
            }
            _ => unreachable!(),
        }
    }

    Ok(EmptyResponse)
}
