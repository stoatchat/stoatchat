use revolt_database::{util::reference::Reference, AdminAuthorization, Database, PartialServer};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// # Change Server Owner
///
/// Change the ownership of a server.
#[openapi(tag = "Admin")]
#[post("/servers/<id>/owner?<case>&<user_id>")]
pub async fn admin_server_change_owner(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    user_id: Reference,
) -> Result<EmptyResponse> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let server = id.as_server(db).await?;
    let target = user_id.as_user(db).await?;

    if target.id == server.owner {
        return Err(create_error!(InvalidOperation));
    }

    let partial = PartialServer {
        owner: Some(target.id),
        ..Default::default()
    };
    db.update_server(&server.id, &partial, vec![]).await?;

    let context = format!("user id: {:}, server id: {:}", &user_id.id, &id.id);
    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::ServerChangeOwner,
        case,
        Some(&id.id),
        Some(&context),
    )
    .await?;

    Ok(EmptyResponse)
}
