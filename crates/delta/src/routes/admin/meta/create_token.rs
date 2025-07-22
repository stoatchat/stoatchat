use iso8601_timestamp::{Duration, Timestamp};
use revolt_database::{AdminMachineToken, AdminToken, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::user_has_permission;

/// Create a token for your account. Must have the CreateTokens permission
#[openapi(tag = "Admin")]
#[post("/tokens", data = "<body>")]
pub async fn admin_create_token(
    db: &State<Database>,
    auth: AdminMachineToken,
    body: Json<v0::AdminTokenCreate>,
) -> Result<Json<v0::AdminToken>> {
    if !user_has_permission(
        &auth.on_behalf_of,
        v0::AdminUserPermissionFlags::CreateTokens,
    ) {
        return Err(create_error!(MissingPermission {
            permission: "CreateTokens".to_string()
        }));
    }

    if body.expiry > Timestamp::now_utc() + Duration::days(30) {
        return Err(create_error!(FailedValidation {
            error: "The expiry is more than 30 days away.".to_string()
        }));
    }

    let token = AdminToken::new(&auth.on_behalf_of.id, body.expiry);
    db.admin_token_create(token.clone()).await?;
    Ok(Json(v0::AdminToken {
        id: token.id,
        user_id: token.user_id,
        token: token.token,
        expiry: body.expiry,
    }))
}
