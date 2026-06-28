use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};
use revolt_database::{util::reference::Reference, AdminAuthorization, Database, EmailVerification, MFATicket};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_database::util::email::normalise_email;

/// Verify the email of an account. Requires ManageAccounts permissions
#[openapi(tag = "Admin")]
#[put("/accounts/email/<target>?<case>")]
pub async fn admin_account_email_verify(
    db: &State<Database>,
    auth: AdminAuthorization,
    target: Reference<'_>,
    case: Option<&str>,
) -> Result<EmptyResponse> {
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

    // Update account email
    if let EmailVerification::Moving { new_email, .. } = &account.verification {
        account.email = new_email.clone();
        account.email_normalised = normalise_email(new_email.clone());
    } else {
        let mut ticket = MFATicket::new(account.id.to_string(), false);
        ticket.authorised = true;
        ticket.save(db).await?;
    };

    // Mark as verified
    account.verification = EmailVerification::Verified;

    // Save to database
    account.save(db).await?;

    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::DeleteAccount,
        case,
        Some(&target.id),
        None,
    )
        .await?;

    Ok(EmptyResponse)
}
