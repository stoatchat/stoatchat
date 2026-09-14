use iso8601_timestamp::{Duration, Timestamp};
use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Database, Invite, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};

use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::util::audit_log_reason::AuditLogReason;

/// # Create Invite
///
/// Creates an invite to this channel.
///
/// Channel must be a `TextChannel`.
#[openapi(tag = "Channel Invites")]
#[post("/<target>/invites", data = "<data>")]
pub async fn create_invite(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    data: Json<v0::DataCreateInvite>,
) -> Result<Json<v0::Invite>> {
    let data = data.into_inner();

    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    const MAX_INVITE_LIFETIME: Duration = Duration::days(30);

    if let Some(expires) = data.expires {
        let now = Timestamp::now_utc();
        if expires <= now || expires > now + MAX_INVITE_LIFETIME {
            return Err(create_error!(InvalidOperation));
        }
    }

    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::InviteOthers)?;

    let invite = Invite::create_channel_invite(db, &user, &channel, data.max_uses, data.expires).await?;

    if let Some(server_id) = channel.server() {
        AuditLogEntryAction::InviteCreate {
            invite: invite.code().to_string(),
            channel: channel.id().to_string(),
        }
        .insert(db, server_id.to_string(), reason, user.id, None)
        .await;
    }

    Ok(Json(invite.into()))
}
