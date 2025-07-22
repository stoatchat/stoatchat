use revolt_database::{util::reference::Reference, AdminAuthorization, Database, Member};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// # Add server member
///
/// Add a user to a server, optionally disabling notifications
#[openapi(tag = "Admin")]
#[post("/servers/<id>/members?<case>&<user_id>&<suppress_alerts>")]
pub async fn admin_server_add_member(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    user_id: Reference,
    suppress_alerts: bool,
) -> Result<Json<v0::MemberWithUserResponse>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let server = id.as_server(db).await?;
    let user = user_id.as_user(db).await?;
    let (member, _) = Member::create(db, &server, &user, None, !suppress_alerts).await?;

    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::ServerAddMember,
        case,
        Some(&id.id),
        None,
    )
    .await?;

    Ok(Json(v0::MemberWithUserResponse {
        user: user.into_self(false).await,
        member: member.into(),
    }))
}
