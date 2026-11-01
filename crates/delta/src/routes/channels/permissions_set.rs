use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{sync_voice_permissions, VoiceClient},
    AuditLogEntryAction, Database, User,
};
use revolt_models::v0;
use revolt_permissions::{ChannelPermission, Override, OverrideField, PermissionQuery, calculate_channel_permissions};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::util::{audit_log_reason::AuditLogReason, permissions};

/// # Set Role Permission
///
/// Sets permissions for the specified role in this channel.
///
/// Channel must be a `TextChannel`.
#[openapi(tag = "Channel Permissions")]
#[put("/<target>/permissions/<role_id>", data = "<data>", rank = 2)]
pub async fn set_role_permissions(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    role_id: String,
    data: Json<v0::DataSetRolePermissions>,
) -> Result<Json<v0::Channel>> {
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions: revolt_permissions::PermissionValue =
        calculate_channel_permissions(&mut query).await;

    query.set_server_from_channel().await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    if let Some(server) = query.server_ref() {
        if let Some(role) = server.roles.get(&role_id) {
            if server.owner != user.id
                && role.rank <= query.get_member_rank().unwrap_or(i64::MIN)
            {
                return Err(create_error!(NotElevated));
            }

            let current_value: Override = role.permissions.into();
            permissions
                .throw_permission_override(current_value, &data.permissions)
                .await?;

            permissions::throw_if_would_lose_channel_view(
                db,
                &user,
                &channel,
                Some(&role_id),
                &data.permissions,
            )
            .await?;

            let mut new_channel = channel.clone();
            let override_field: OverrideField = data.permissions.clone().into();
            let server_id = server.id.clone();

            new_channel
                .set_role_permission(db, &role_id, data.permissions.clone().into())
                .await?;

            sync_voice_permissions(db, voice_client, &new_channel, Some(server), Some(&role_id))
                .await?;

            AuditLogEntryAction::ChannelRolePermissionsEdit {
                channel: new_channel.id().to_string(),
                role: role_id,
                permissions: override_field,
            }
            .insert(db, server_id, reason, user.id, None)
            .await;

            Ok(Json(new_channel.into()))
        } else {
            Err(create_error!(NotFound))
        }
    } else {
        Err(create_error!(InvalidOperation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test::TestHarness;
    use revolt_database::{Member, PartialMember, PartialRole, PartialServer, Server};
    use revolt_models::v0;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{Header, Status};

    /// Sets up a server where the acting member only sees the channel through
    /// the given role, so that denying `ViewChannel` on it locks them out.
    async fn lockout_setup(
        harness: &TestHarness,
        server: &mut Server,
        user: &User,
    ) -> (revolt_database::Channel, revolt_database::Role) {
        server
            .update(
                &harness.db,
                PartialServer {
                    default_permissions: Some(0),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to clear default permissions");

        let channel = harness.new_channel(server).await;

        let (mut member, _) = Member::create(&harness.db, server, user, None)
            .await
            .expect("Failed to add admin member");

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
                    roles: Some(vec![manage_role.id, view_role.id.clone()]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to assign roles");

        (channel, view_role)
    }

    fn deny_view() -> v0::DataSetRolePermissions {
        v0::DataSetRolePermissions {
            permissions: Override {
                allow: 0,
                deny: ChannelPermission::ViewChannel as u64,
            },
        }
    }

    #[rocket::async_test]
    async fn cannot_deny_own_view_via_role_permissions() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, admin) = harness.new_user().await;

        let (mut server, _) = harness.new_server(&owner).await;
        let (channel, view_role) = lockout_setup(&harness, &mut server, &admin).await;

        let res = harness
            .client
            .put(format!(
                "/channels/{}/permissions/{}",
                channel.id(),
                view_role.id
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
    async fn owner_can_deny_own_view_via_role_permissions() {
        let harness = TestHarness::new().await;
        let (_, session, owner) = harness.new_user().await;

        let (server, _) = harness.new_server(&owner).await;

        // The owner just needs an arbitrary role to edit.
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
        let channel = harness.new_channel(&server).await;

        let res = harness
            .client
            .put(format!("/channels/{}/permissions/{}", channel.id(), role.id))
            .header(Header::new("x-session-token", session.token))
            .json(&deny_view())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
    }
}

