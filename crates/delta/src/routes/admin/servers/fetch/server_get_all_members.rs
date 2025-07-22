use futures::future::join_all;
use iso8601_timestamp::Timestamp;
use revolt_database::{util::reference::Reference, AdminAuthorization, Database};
use revolt_models::v0::{self};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};

/// Get a list of admin users. Any active user may use this endpoint.
/// Typically the client should cache this data.
#[openapi(tag = "Admin")]
#[get("/servers/<id>/members?<case>&<after>")]
pub async fn admin_server_get_members(
    db: &State<Database>,
    auth: AdminAuthorization,
    id: Reference,
    case: Option<&str>,
    after: Option<usize>,
) -> Result<Json<v0::AdminMemberWithUserAndOffsetResponse>> {
    let user = flatten_authorized_user(&auth);
    if !user_has_permission(user, v0::AdminUserPermissionFlags::ManageServers) {
        return Err(create_error!(MissingPermission {
            permission: "ManageServers".to_string()
        }));
    }

    let server = id.as_server(db).await?;
    let members = db.fetch_server_members(&server.id, 100, after).await?;

    create_audit_action(
        db,
        &user.id,
        v0::AdminAuditItemActions::ServerFetchMembers,
        case,
        Some(&id.id),
        None,
    )
    .await?;

    let members_and_users = join_all(members.iter().map(|(u, m)| async move {
        let user: v0::User = u.clone().into_self(false).await;
        let member: v0::Member = m.clone().into();
        v0::MemberWithUserResponse { user, member }
    }))
    .await;

    let mut resp = v0::AdminMemberWithUserAndOffsetResponse {
        after: None,
        users: vec![],
    };

    if let Some(last) = members_and_users.last() {
        resp.after = Some(
            last.member
                .joined_at
                .duration_since(Timestamp::UNIX_EPOCH)
                .whole_milliseconds() as usize,
        );
    }

    resp.users = members_and_users;

    Ok(Json(resp))
}
