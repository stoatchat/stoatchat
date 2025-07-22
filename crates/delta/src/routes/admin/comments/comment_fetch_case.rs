use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// Fetch all comments related to a case. id should be the 7 character short code. Requires Comments permission.
#[openapi(tag = "Admin")]
#[get("/comments/case/<id>")]
pub async fn admin_comment_fetch_case(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
) -> Result<Json<Vec<v0::AdminComment>>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::Comments) {
        return Err(create_error!(MissingPermission {
            permission: "Comments".to_string()
        }));
    }

    let comments: Vec<v0::AdminComment> = db
        .admin_comment_fetch_object_comments(&id.id)
        .await?
        .iter()
        .map(|f| f.clone().into())
        .collect();

    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::CommentFetchForObject,
        Some(&id.id),
        None,
        None,
    )
    .await?;

    Ok(Json(comments))
}
