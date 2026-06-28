use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};
use revolt_database::util::reference::Reference;
use revolt_database::{AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_database::util::email::{normalise_email, validate_email};

/// Change the email of an account. Requires ManageAccounts permissions
#[openapi(tag = "Admin")]
#[patch("/accounts/email/<target>?<case>", data = "<data>")]
pub async fn admin_account_email_change(
    db: &State<Database>,
    auth: AdminAuthorization,
    target: Reference<'_>,
    case: Option<&str>,
    data: Json<v0::DataChangeEmail>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    validate_email(&data.email)?;

    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageAccounts) {
        return Err(create_error!(MissingPermission {
            permission: "ManageAccounts".to_string()
        }));
    }

    let target = target.as_user(db).await?;

    if target.privileged {
        return Err(create_error!(PrivilegedAccount));
    }

    let admin = db.admin_user_fetch(&target.id).await.ok();

    if let Some(admin) = admin {
        if user_has_permission(&admin, v0::AdminUserPermissionFlags::ManageAccounts) {
            return Err(create_error!(PrivilegedAccount));
        }
    }

    let mut account = db.fetch_account(&target.id).await?;

    account.email_normalised = normalise_email(data.email.clone());
    account.email = data.email;
    account.save(db).await?;


    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::EmailChange,
        case,
        Some(&target.id),
        None,
    )
        .await?;

    Ok(EmptyResponse)
}
