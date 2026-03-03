use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{sync_voice_permissions, VoiceClient},
    AuditLogEntryAction, Database, PartialRole, User,
};
use revolt_models::v0;
use revolt_permissions::{
    calculate_server_permissions, ChannelPermission, Override, OverrideField,
};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::util::audit_log_reason::AuditLogReason;

/// # Set Role Permission
///
/// Sets permissions for the specified role in the server.
#[openapi(tag = "Server Permissions")]
#[put("/<target>/permissions/<role_id>", data = "<data>", rank = 2)]
pub async fn set_role_permission(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    role_id: String,
    data: Json<v0::DataSetServerRolePermission>,
) -> Result<Json<v0::Server>> {
    let data = data.into_inner();

    let mut server = target.as_server(db).await?;

    let (current_value, rank) = server
        .roles
        .get(&role_id)
        .map(|x| (x.permissions, x.rank))
        .ok_or_else(|| create_error!(NotFound))?;

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    let permissions = calculate_server_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    // Prevent us from editing roles above us
    if rank <= query.get_member_rank().unwrap_or(i64::MIN) {
        return Err(create_error!(NotElevated));
    }

    // Ensure we have access to grant these permissions forwards
    let current_override: Override = current_value.into();
    permissions
        .throw_permission_override(current_override, &data.permissions)
        .await?;

    let override_field: OverrideField = data.permissions.into();

    server
        .set_role_permission(db, &role_id, override_field)
        .await?;

    AuditLogEntryAction::RoleEdit {
        role: role_id.clone(),
        remove: Vec::new(),
        before: PartialRole {
            permissions: Some(current_value),
            ..Default::default()
        },
        after: PartialRole {
            permissions: Some(override_field),
            ..Default::default()
        },
    }
    .insert(db, server.id.clone(), reason.0, user.id)
    .await;

    for channel_id in &server.channels {
        let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

        sync_voice_permissions(db, voice_client, &channel, Some(&server), Some(&role_id)).await?;
    }

    Ok(Json(server.into()))
}
