use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// Delete Server Invite
///
/// Delete a server invite, or all of a server's invites.
#[openapi(tag = "Admin")]
#[delete("/servers/<id>/invites?<case>&<slug>")]
pub async fn admin_server_delete_invites(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    slug: Option<Reference>,
) -> Result<EmptyResponse> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    if let Some(slug) = slug {
        let invite = slug.as_invite(db).await?;

        db.delete_invite(&slug.id).await?;

        let context = format!("server id: {:}, invite slug: {:}", &id.id, invite.code());
        create_audit_action(
            db,
            &user.id,
            v0::AdminAuditItemActions::ServerDeleteInvite,
            case,
            Some(&id.id),
            Some(&context),
        )
        .await?;
    } else {
        let invites = db.fetch_invites_for_server(&id.id).await?;
        for invite in invites {
            db.delete_invite(invite.code()).await?;
        }
        create_audit_action(
            db,
            &user.id,
            v0::AdminAuditItemActions::ServerDeleteAllInvites,
            case,
            Some(&id.id),
            None,
        )
        .await?;
    }

    Ok(EmptyResponse)
}
