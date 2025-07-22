use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// Get a list of admin users. Any active user may use this endpoint.
/// Typically the client should cache this data.
#[openapi(tag = "Admin")]
#[post(
    "/servers/<id>/unban?<case>&<user_id>&<suppress_alerts>",
    data = "<body>"
)]
pub async fn admin_server_unban_member(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    user_id: Reference,
    suppress_alerts: bool,
    body: Json<v0::DataBanCreate>,
) -> Result<EmptyResponse> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let ban = db
        .fetch_ban(&id.id, &user_id.id)
        .await
        .map_err(|_| create_error!(NotFound))?;

    db.delete_ban(&ban.id).await?;

    let context = format!(
        "user id: {:}, server id: {:}, reason: {:}",
        &user_id.id,
        &id.id,
        body.reason
            .clone()
            .unwrap_or_else(|| "None Provided".to_string())
    );
    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::ServerUnbanMember,
        case,
        Some(&id.id),
        Some(&context),
    )
    .await?;

    Ok(EmptyResponse)
}
