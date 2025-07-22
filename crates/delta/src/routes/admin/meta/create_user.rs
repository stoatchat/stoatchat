use revolt_database::{AdminAuthorization, AdminUser, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::{flatten_authorized_user, user_has_permission};

/// Edit an admin user. Requires ManageAdminUsers flag.
#[openapi(tag = "Admin")]
#[post("/users", data = "<body>")]
pub async fn admin_create_user(
    db: &State<Database>,
    auth: AdminAuthorization,
    body: Json<v0::AdminUserCreate>,
) -> Result<Json<v0::AdminUser>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageAdminUsers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageAdminUsers".to_string()
        }));
    }

    // TODO: technically there's a privilege escalation here since anyone with the manageAdminUsers permission can assign whatever permissions they want.
    // But this is built with the assumption that people with ManageAdminUsers are already privileged.
    let user = AdminUser::new(&body.email, &body.platform_user_id, body.permissions);
    db.admin_user_insert(user.clone()).await?;
    Ok(Json(user.into()))
}
