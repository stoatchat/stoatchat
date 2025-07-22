use revolt_database::{util::reference::Reference, AdminAuthorization, Database, Invite};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// # Create Server Invite
///
/// Create an invite to a server. Optionally include a slug to create a vanity URL.
#[openapi(tag = "Admin")]
#[post("/servers/<id>/invites?<case>&<slug>&<channel_id>")]
pub async fn admin_server_create_invite(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    channel_id: Reference,
    case: Option<&str>,
    slug: Option<&str>,
) -> Result<Json<v0::Invite>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let server = id.as_server(db).await?;
    let channel = channel_id.as_channel(db).await?;
    let creator = db.fetch_user(&server.owner).await?;

    let invite: Invite;

    if let Some(slug) = slug {
        invite = Invite::Server {
            code: slug.to_string(),
            creator: creator.id.clone(),
            server: server.id.clone(),
            channel: channel.id().to_string(),
        };
        #[allow(clippy::disallowed_methods)]
        db.insert_invite(&invite).await?;
    } else {
        invite = Invite::create_channel_invite(db, &creator, &channel).await?;
    }

    let context = format!(
        "server id: {:}, invite slug: {:}, channel: {:}",
        &id.id,
        invite.code(),
        channel.id()
    );
    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::ServerCreateInvite,
        case,
        Some(&id.id),
        Some(&context),
    )
    .await?;

    Ok(Json(invite.into()))
}
