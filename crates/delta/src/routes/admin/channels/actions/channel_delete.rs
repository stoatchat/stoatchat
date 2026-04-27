use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};
use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

#[openapi(tag = "Admin")]
#[delete("/admin/channels/<channel_id>?<case>")]
pub async fn admin_delete_channel(
    db: &State<Database>,
    auth: AdminAuthorization,
    channel_id: Reference<'_>,
    case: Option<&str>,
) -> Result<EmptyResponse> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageChannels) {
        return Err(create_error!(MissingPermission {
            permission: "ManageChannels".to_string()
        }));
    }

    let target = channel_id.as_channel(&db).await?;

    target.delete(&db).await?;

    create_audit_action(
        &db,
        &user.id,
        v0::AdminAuditItemActions::DeleteChannel,
        case,
        Some(channel_id.id),
        None,
    )
    .await?;

    Ok(EmptyResponse)
}
