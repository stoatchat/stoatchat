use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{sync_voice_permissions, VoiceClient},
    AuditLogEntryAction, Database, PartialRole, User,
};
use revolt_models::v0;
use revolt_permissions::{
    calculate_server_permissions, ChannelPermission, Override, OverrideField,
};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::util::{audit_log_reason::AuditLogReason, permissions};

/// # Set Role Permission
///
/// Sets permissions for the specified role in the server.
#[openapi(tag = "Server Permissions")]
#[put("/<target>/permissions/<role_id>", data = "<data>", rank = 2)]
pub async fn set_role_permission(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    role_id: String,
    data: Json<v0::DataSetServerRolePermission>,
) -> Result<Json<v0::Server>> {
    let data = data.into_inner();

    let mut server = target.as_server(db).await?;

    let (current_value, rank) = server
        .roles
        .get(&role_id)
        .map(|x| (x.permissions, x.rank))
        .ok_or_else(|| create_error!(NotFound))?;

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    let permissions = calculate_server_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    // Prevent us from editing roles above us
    if rank <= query.get_member_rank().unwrap_or(i64::MIN) {
        return Err(create_error!(NotElevated));
    }

    // Ensure we have access to grant these permissions forwards
    let current_override: Override = current_value.into();
    permissions
        .throw_permission_override(current_override, &data.permissions)
        .await?;

    permissions::throw_if_would_lose_server_channel_view(
        db,
        &user,
        &server,
        &role_id,
        &data.permissions,
    )
    .await?;

    let override_field: OverrideField = data.permissions.into();

    server
        .set_role_permission(db, &role_id, override_field)
        .await?;

    AuditLogEntryAction::RoleEdit {
        role: role_id.clone(),
        before: PartialRole {
            permissions: Some(current_value),
            ..Default::default()
        },
        after: PartialRole {
            permissions: Some(override_field),
            ..Default::default()
        },
    }
    .insert(db, server.id.clone(), reason, user.id, None)
    .await;

    for channel_id in &server.channels {
        let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

        sync_voice_permissions(db, voice_client, &channel, Some(&server), Some(&role_id)).await?;
    }

    Ok(Json(server.into(db).await))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test::TestHarness;
    use revolt_database::{Member, PartialMember, PartialRole, Role, Server};
    use revolt_result::{Error, ErrorType};
    use rocket::http::{Header, Status};

    /// Gives the member a managing role and a separate view role, pushing the
    /// view role below the managing role so it can be edited.
    async fn view_and_manage_roles(
        harness: &TestHarness,
        server: &Server,
        user: &User,
    ) -> (Role, Role) {
        let (mut member, _) = Member::create(&harness.db, server, user, None)
            .await
            .expect("Failed to add member");

        let manage_role = harness
            .new_role(
                server,
                0,
                Some(OverrideField {
                    a: ChannelPermission::ManagePermissions as i64,
                    d: 0,
                }),
            )
            .await;

        let mut view_role = harness
            .new_role(
                server,
                1,
                Some(OverrideField {
                    a: ChannelPermission::ViewChannel as i64,
                    d: 0,
                }),
            )
            .await;
        view_role
            .update(
                &harness.db,
                &server.id,
                PartialRole {
                    rank: Some(1),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to re-rank view role");

        member
            .update(
                &harness.db,
                PartialMember {
                    roles: Some(vec![manage_role.id.clone(), view_role.id.clone()]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to assign roles");

        (manage_role, view_role)
    }

    fn deny_view() -> v0::DataSetServerRolePermission {
        v0::DataSetServerRolePermission {
            permissions: Override {
                allow: 0,
                deny: ChannelPermission::ViewChannel as u64,
            },
        }
    }

    #[rocket::async_test]
    async fn cannot_deny_own_view_via_server_role_permissions() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, admin) = harness.new_user().await;

        let (server, _) = harness.new_server(&owner).await;
        harness.new_channel(&server).await;
        let (_, view_role) = view_and_manage_roles(&harness, &server, &admin).await;

        let res = harness
            .client
            .put(format!(
                "/servers/{}/permissions/{}",
                server.id, view_role.id
            ))
            .header(Header::new("x-session-token", session.token))
            .json(&deny_view())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Forbidden);
        assert!(matches!(
            res.into_json::<Error>().await.unwrap().error_type,
            ErrorType::CannotRemoveOwnChannelView
        ));
    }

    #[rocket::async_test]
    async fn owner_can_deny_own_view_via_server_role_permissions() {
        let harness = TestHarness::new().await;
        let (_, session, owner) = harness.new_user().await;

        let (server, _) = harness.new_server(&owner).await;
        harness.new_channel(&server).await;
        let role = harness
            .new_role(
                &server,
                1,
                Some(OverrideField {
                    a: ChannelPermission::ViewChannel as i64,
                    d: 0,
                }),
            )
            .await;

        let res = harness
            .client
            .put(format!("/servers/{}/permissions/{}", server.id, role.id))
            .header(Header::new("x-session-token", session.token))
            .json(&deny_view())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
    }
}

