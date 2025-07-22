use revolt_database::{AdminAuthorization, AdminComment, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// Create a comment on an object or case. Requires Comments permission.
#[openapi(tag = "Admin")]
#[post("/comments", data = "<body>")]
pub async fn admin_comment_create(
    db: &State<Database>,
    auth: AdminAuthorization,
    body: Json<v0::AdminCommentCreate>,
) -> Result<Json<v0::AdminComment>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::Comments) {
        return Err(create_error!(MissingPermission {
            permission: "Comments".to_string()
        }));
    }

    let comment = AdminComment::new(
        &body.object_id,
        &user.id,
        &body.content,
        body.case_id.as_deref(),
    );
    db.admin_comment_insert(comment.clone()).await?;

    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::CommentCreate,
        None,
        Some(&body.object_id),
        None,
    )
    .await?;

    Ok(Json(comment.into()))
}
