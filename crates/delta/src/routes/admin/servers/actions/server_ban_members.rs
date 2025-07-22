use revolt_database::{util::reference::Reference, AdminAuthorization, Database, ServerBan};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// # Ban a server member
///
/// Ban a member who is not the owner
#[openapi(tag = "Admin")]
#[post(
    "/servers/<id>/ban?<case>&<user_id>&<suppress_alerts>",
    data = "<body>"
)]
pub async fn admin_server_ban_member(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    user_id: Reference,
    suppress_alerts: bool,
    body: Json<v0::DataBanCreate>,
) -> Result<Json<v0::ServerBan>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let body = body.into_inner();
    body.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let server = id.as_server(db).await?;
    let target = user_id.as_user(db).await?;

    if target.id == server.owner {
        return Err(create_error!(InvalidOperation));
    }

    let resp = if let Ok(member) = user_id.as_member(db, &id.id).await {
        member
            .remove(
                db,
                &server,
                revolt_database::RemovalIntention::Ban,
                suppress_alerts,
            )
            .await?;

        Ok(ServerBan::create(db, &server, &target.id, body.reason.clone()).await?)
    } else {
        Err(create_error!(NotAMember))
    }?;

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
        v0::AdminAuditItemActions::ServerBanMember,
        case,
        Some(&id.id),
        Some(&context),
    )
    .await?;

    Ok(Json(resp.into()))
}
