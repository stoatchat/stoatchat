use revolt_database::{util::reference::Reference, AdminMachineToken, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::user_has_permission;

/// Revoke a token. Must be your own, or you must have the ManageAdminUsers permission to revoke other's tokens.
/// You must have the CreateTokens permission.
#[openapi(tag = "Admin")]
#[delete("/tokens/<token>")]
pub async fn admin_revoke_token(
    db: &State<Database>,
    auth: AdminMachineToken,
    token: Reference,
) -> Result<EmptyResponse> {
    if !user_has_permission(
        &auth.on_behalf_of,
        v0::AdminUserPermissionFlags::CreateTokens,
    ) {
        return Err(create_error!(MissingPermission {
            permission: "CreateTokens".to_string()
        }));
    }

    let token = token.as_admin_token(db).await?;

    // only own user and those with ManageAdminUsers can revoke a token
    if token.user_id != auth.on_behalf_of.id
        && !user_has_permission(
            &auth.on_behalf_of,
            v0::AdminUserPermissionFlags::ManageAdminUsers,
        )
    {
        return Err(create_error!(NotFound));
    }

    db.admin_token_revoke(&token.id).await?;
    Ok(EmptyResponse)
}
