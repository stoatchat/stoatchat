use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// # Ban a server member
///
/// Ban a member who is not the owner
#[openapi(tag = "Admin")]
#[post("/servers/<id>/remove?<case>&<user_id>&<suppress_alerts>")]
pub async fn admin_server_remove_members(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    user_id: Vec<Reference>,
    suppress_alerts: bool,
) -> Result<EmptyResponse> {
    let admin_user = flatten_authorized_user(&auth);
    if !user_has_permission(admin_user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let server = id.as_server(db).await?;
    for uid in user_id {
        let target = uid.as_member(db, &id.id).await?;

        if target.id.user == server.owner {
            return Err(create_error!(InvalidOperation));
        }

        target
            .remove(
                db,
                &server,
                revolt_database::RemovalIntention::Kick,
                suppress_alerts,
            )
            .await?;

        let context = format!("user id: {:}, server id: {:}", &uid.id, &id.id);
        create_audit_action(
            db,
            &admin_user.id,
            v0::AdminAuditItemActions::ServerRemoveMember,
            case,
            Some(&id.id),
            Some(&context),
        )
        .await?;
    }

    Ok(EmptyResponse)
}
