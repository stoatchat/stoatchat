use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// # Edit server
///
/// Edit any attributes of a server.
#[openapi(tag = "Admin")]
#[patch("/servers/<id>?<case>", data = "<body>")]
pub async fn admin_server_edit(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    body: Json<v0::DataEditServer>,
) -> Result<Json<v0::Server>> {
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

    let mut server = id.as_server(db).await?;
    let user = db.fetch_user(&server.owner).await?;
    crate::routes::servers::server_edit::edit_data(body, db, &mut server, &user).await?;

    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::ServerEdit,
        case,
        Some(&id.id),
        None,
    )
    .await?;

    Ok(Json(server.into()))
}
