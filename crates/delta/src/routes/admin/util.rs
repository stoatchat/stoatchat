use revolt_database::{
    AdminAuditItem, AdminAuthorization, AdminComment, AdminUser, AdminUserPermissionFlagsValue,
    Database,
};
use revolt_models::v0::{AdminAuditItemActions, AdminUserPermissionFlags};
use revolt_result::Result;

pub fn user_has_permission(user: &AdminUser, permission: AdminUserPermissionFlags) -> bool {
    let flags = AdminUserPermissionFlagsValue(user.permissions);

    flags.has(permission)
}

pub fn flatten_authorized_user(auth: &AdminAuthorization) -> &AdminUser {
    match auth {
        AdminAuthorization::AdminUser(admin_user) => admin_user,
        AdminAuthorization::AdminMachine(admin_machine_token) => &admin_machine_token.on_behalf_of,
    }
}

/// Creates audit logs, and if applicable, adds a comment to the referenced case.
pub async fn create_audit_action(
    db: &Database,
    mod_id: &str,
    action: AdminAuditItemActions,
    short_case_id: Option<&str>,
    target_id: Option<&str>,
    context: Option<&str>,
) -> Result<()> {
    let audit = AdminAuditItem::new(mod_id, action.clone(), short_case_id, target_id, context);
    db.admin_audit_insert(audit).await?;

    if action.makes_comment() {
        if let Some(short_case_id) = short_case_id {
            if let Some(target_id) = target_id {
                let case = db.admin_case_fetch_from_shorthand(short_case_id).await?;
                db.admin_comment_insert(AdminComment::new_system_message(
                    &case.id,
                    mod_id,
                    short_case_id,
                    action,
                    context,
                    target_id,
                ))
                .await?;
            }
        }
    }
    Ok(())
}
