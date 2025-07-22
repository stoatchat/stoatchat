use iso8601_timestamp::Timestamp;
use revolt_database::{
    util::reference::Reference, AdminAuthorization, Database, PartialAdminComment,
};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use rocket_empty::EmptyResponse;

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// Create a comment on an object or case. Requires Comments permission.
#[openapi(tag = "Admin")]
#[patch("/comments/<id>", data = "<body>")]
pub async fn admin_comment_edit(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    body: Json<v0::AdminCommentEdit>,
) -> Result<EmptyResponse> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::Comments) {
        return Err(create_error!(MissingPermission {
            permission: "Comments".to_string()
        }));
    }

    let existing = db.admin_comment_fetch(&id.id).await?;
    if existing.user_id != user.id {
        return Err(create_error!(MissingPermission {
            permission: "CannotEditOthersComment".to_string()
        }));
    }

    let partial = PartialAdminComment {
        content: Some(body.content.clone()),
        edited_at: Some(Timestamp::now_utc().format_short().to_string()),
        ..Default::default()
    };
    db.admin_comment_update(&id.id, &partial).await?;

    let context = format!("comment id {:}, object id {:}", &id.id, &existing.object_id);
    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::CommentEdit,
        None,
        Some(&existing.object_id),
        Some(&context),
    )
    .await?;

    Ok(EmptyResponse)
}
