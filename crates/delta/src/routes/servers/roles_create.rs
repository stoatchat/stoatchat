use revolt_config::config;
use revolt_database::{
    AuditLogEntryAction, Database, Role, User, util::{permissions::DatabasePermissionQuery, reference::Reference}
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

use crate::util::audit_log_reason::AuditLogReason;

/// # Create Role
///
/// Creates a new server role.
#[openapi(tag = "Server Permissions")]
#[post("/<target>/roles", data = "<data>")]
pub async fn create(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    data: Json<v0::DataCreateRole>,
) -> Result<Json<v0::NewRoleResponse>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageRole)?;

    let config = config().await;
    if server.roles.len() >= config.features.limits.global.server_roles {
        return Err(create_error!(TooManyRoles {
            max: config.features.limits.global.server_roles,
        }));
    };

    let role = Role {
        name: data.name,
        // Rank of the new role should be below the lowest role
        rank: server.roles.len() as i64,
        colour: None,
        hoist: false,
        permissions: Default::default(),
    };

    let role_id = role.create(db, &server.id).await?;

    AuditLogEntryAction::RoleCreate { role: role_id.clone(), name: role.name.clone() }
        .insert(db, server.id, reason.0, user.id)
        .await;

    Ok(Json(v0::NewRoleResponse {
        id: role_id,
        role: role.into(),
    }))
}
