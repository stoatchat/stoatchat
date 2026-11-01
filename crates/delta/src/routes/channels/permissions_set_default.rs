use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{sync_voice_permissions, VoiceClient},
    AuditLogEntryAction, Channel, Database, PartialChannel, User,
};
use revolt_models::v0::{self, DataDefaultChannelPermissions};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use crate::util::{audit_log_reason::AuditLogReason, permissions};

/// # Set Default Permission
///
/// Sets permissions for the default role in this channel.
///
/// Channel must be a `Group` or `TextChannel`.
#[openapi(tag = "Channel Permissions")]
#[put("/<target>/permissions/default", data = "<data>", rank = 1)]
pub async fn set_default_channel_permissions(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    data: Json<v0::DataDefaultChannelPermissions>,
) -> Result<Json<v0::Channel>> {
    let data = data.into_inner();

    let mut channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    match &channel {
        Channel::Group { .. } => {
            if let DataDefaultChannelPermissions::Value { permissions } = data {
                channel
                    .update(
                        db,
                        PartialChannel {
                            permissions: Some(permissions as i64),
                            ..Default::default()
                        },
                        vec![],
                    )
                    .await?;
            } else {
                return Err(create_error!(InvalidOperation));
            }
        }
        Channel::TextChannel {
            id,
            server,
            default_permissions,
            ..
        } => {
            if let DataDefaultChannelPermissions::Field { permissions: field } = data {
                permissions
                    .throw_permission_override(default_permissions.map(|x| x.into()), &field)
                    .await?;

                permissions::throw_if_would_lose_channel_view(
                    db, &user, &channel, None, &field,
                )
                .await?;

                let partial = PartialChannel {
                    default_permissions: Some(field.into()),
                    ..Default::default()
                };

                let id = id.clone();
                let server = server.clone();

                let before = channel.generate_diff(&partial, &[]);

                channel.update(db, partial.clone(), vec![]).await?;

                AuditLogEntryAction::ChannelEdit {
                    channel: id,
                    before,
                    after: partial,
                }
                .insert(db, server, reason, user.id, None)
                .await;
            } else {
                return Err(create_error!(InvalidOperation));
            }
        }
        _ => return Err(create_error!(InvalidOperation)),
    }

    let server = match channel.server() {
        Some(server_id) => Some(Reference::from_unchecked(server_id).as_server(db).await?),
        None => None,
    };

    sync_voice_permissions(db, voice_client, &channel, server.as_ref(), None).await?;

    Ok(Json(channel.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test::TestHarness;
    use revolt_database::{Member, PartialMember};
    use revolt_models::v0;
    use revolt_permissions::Override;
    use revolt_result::{Error, ErrorType};
    use rocket::http::{Header, Status};

    /// Build a non-owner member with a role that lets them manage permissions
    /// on the server's channels, without granting them `ViewChannel`.
    async fn admin_member(
        harness: &TestHarness,
        server: &revolt_database::Server,
        user: &User,
        allow: u64,
    ) -> Member {
        let (mut member, _) = Member::create(&harness.db, server, user, None)
            .await
            .expect("Failed to add admin member");

        let role = harness
            .new_role(
                server,
                1,
                Some(revolt_permissions::OverrideField {
                    a: allow as i64,
                    d: 0,
                }),
            )
            .await;

        member
            .update(
                &harness.db,
                PartialMember {
                    roles: Some(vec![role.id]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to assign admin role");

        member
    }

    fn deny_view() -> v0::DataDefaultChannelPermissions {
        v0::DataDefaultChannelPermissions::Field {
            permissions: Override {
                allow: 0,
                deny: ChannelPermission::ViewChannel as u64,
            },
        }
    }

    #[rocket::async_test]
    async fn cannot_deny_own_view_via_default_permissions() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, admin) = harness.new_user().await;

        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;

        admin_member(
            &harness,
            &server,
            &admin,
            ChannelPermission::ManagePermissions as u64,
        )
        .await;

        let res = harness
            .client
            .put(format!("/channels/{}/permissions/default", channel.id()))
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
    async fn owner_can_deny_own_view_via_default_permissions() {
        let harness = TestHarness::new().await;
        let (_, session, owner) = harness.new_user().await;

        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;

        let res = harness
            .client
            .put(format!("/channels/{}/permissions/default", channel.id()))
            .header(Header::new("x-session-token", session.token))
            .json(&deny_view())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn can_deny_default_view_when_still_able_to_view() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, admin) = harness.new_user().await;

        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;

        // Grant the admin their own `ViewChannel` so denying the default
        // permission does not lock them out.
        admin_member(
            &harness,
            &server,
            &admin,
            ChannelPermission::ManagePermissions as u64 | ChannelPermission::ViewChannel as u64,
        )
        .await;

        let res = harness
            .client
            .put(format!("/channels/{}/permissions/default", channel.id()))
            .header(Header::new("x-session-token", session.token))
            .json(&deny_view())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
    }
}
