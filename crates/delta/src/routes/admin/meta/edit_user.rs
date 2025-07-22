use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::{flatten_authorized_user, user_has_permission};

/// Edit an admin user. Requires ManageAdminUsers flag.
#[openapi(tag = "Admin")]
#[patch("/users/<target>", data = "<body>")]
pub async fn admin_edit_user(
    db: &State<Database>,
    auth: AdminAuthorization,
    target: Reference,
    body: Json<v0::AdminUserEdit>,
) -> Result<EmptyResponse> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageAdminUsers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageAdminUsers".to_string()
        }));
    }

    // TODO: technically there's a privilege escalation here since anyone with the manageAdminUsers permission can assign whatever permissions they want.
    // But this is built with the assumption that people with ManageAdminUsers are already privileged.
    db.admin_user_update(&target.id, body.0.into()).await?;
    Ok(EmptyResponse)
}
