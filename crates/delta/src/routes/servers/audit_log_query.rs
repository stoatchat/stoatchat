use std::collections::HashSet;

use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, AuditLogQuery, Database, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

#[openapi(tag = "Audit Logs")]
#[get("/<target>/audit_logs?<options..>")]
pub async fn query(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    options: v0::OptionsAuditLogQuery,
) -> Result<Json<v0::AuditLogQueryResponse>> {
    options.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let server = target.as_server(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ViewAuditLogs)?;

    let v0::OptionsAuditLogQuery {
        user: user_filter,
        r#type,
        before,
        after,
        limit,
        include_users,
    } = options;

    let audit_logs = db
        .get_server_audit_logs(
            &server.id,
            AuditLogQuery {
                user: user_filter,
                r#type,
                before,
                after,
                limit,
            },
        )
        .await?;

    if include_users == Some(true) {
        let mut user_ids = HashSet::new();

        for entry in &audit_logs {
            user_ids.insert(entry.user.clone());

            match &entry.action {
                AuditLogEntryAction::MessageDelete { author, .. } => {
                    user_ids.insert(author.clone());
                }
                AuditLogEntryAction::BanCreate { user } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::BanDelete { user } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::ChannelCreate { .. } => {}
                AuditLogEntryAction::MemberEdit { user, .. } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::MemberKick { user } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::ServerEdit { .. } => {}
                AuditLogEntryAction::RoleEdit { .. } => {}
                AuditLogEntryAction::RoleCreate { .. } => {}
                AuditLogEntryAction::RoleDelete { .. } => {}
                AuditLogEntryAction::RolesReorder { .. } => {}
                AuditLogEntryAction::MessageBulkDelete { .. } => {}
                AuditLogEntryAction::ChannelEdit { .. } => {}
                AuditLogEntryAction::ChannelRolePermissionsEdit { .. } => {}
                AuditLogEntryAction::ChannelDelete { .. } => {}
                AuditLogEntryAction::InviteDelete { .. } => {}
                AuditLogEntryAction::WebhookCreate { .. } => {}
                AuditLogEntryAction::WebhookDelete { .. } => {}
                AuditLogEntryAction::EmojiDelete { .. } => {}
            };
        }

        let user_ids = user_ids.into_iter().collect::<Vec<_>>();

        let users = User::fetch_many_ids_as_mutuals(db, &user, &user_ids).await?;
        let members = db.fetch_members(&server.id, &user_ids).await?;

        Ok(Json(v0::AuditLogQueryResponse::AuditLogsAndUsers {
            audit_logs: audit_logs.into_iter().map(Into::into).collect(),
            users,
            members: members.into_iter().map(Into::into).collect(),
        }))
    } else {
        Ok(Json(v0::AuditLogQueryResponse::AuditLogs(
            audit_logs.into_iter().map(Into::into).collect(),
        )))
    }
}

#[cfg(test)]
mod test {
    use revolt_database::Server;
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    use crate::util::test::TestHarness;

    #[rocket::async_test]
    async fn audit_log_query_without_users() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Test Server".to_string(),
                ..Default::default()
            },
            &user,
            true,
        )
        .await
        .expect("Failed to create test server.");

        let channel = &channels[0];

        let status = harness
            .client
            .delete(format!("/channels/{}", channel.id()))
            .header(Header::new("X-Audit-Log-Reason", "Test Reason"))
            .header(Header::new("x-session-token", session.token.clone()))
            .dispatch()
            .await
            .status();

        assert_eq!(status, Status::NoContent);

        let response = harness
            .client
            .get(format!("/servers/{}/audit_logs", &server.id))
            .header(Header::new("x-session-token", session.token.clone()))
            .dispatch()
            .await
            .into_json::<v0::AuditLogQueryResponse>()
            .await
            .expect("Failed to deserialise audit_logs response");

        let v0::AuditLogQueryResponse::AuditLogs(entries) = response else {
            panic!("Response included users when shouldnt")
        };

        assert_eq!(entries.len(), 1);

        let entry = &entries[0];

        assert_eq!(entry.reason.as_deref(), Some("Test Reason"));
        assert_eq!(&entry.server, &server.id);
        assert_eq!(&entry.user, &user.id);
        assert_eq!(
            &entry.action,
            &v0::AuditLogEntryAction::ChannelDelete {
                channel: channel.id().to_string(),
                name: "General".to_string()
            }
        );
    }

    #[rocket::async_test]
    async fn audit_log_query_with_users() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Test Server".to_string(),
                ..Default::default()
            },
            &user,
            true,
        )
        .await
        .expect("Failed to create test server.");

        let channel = &channels[0];

        let status = harness
            .client
            .delete(format!("/channels/{}", channel.id()))
            .header(Header::new("X-Audit-Log-Reason", "Test Reason"))
            .header(Header::new("x-session-token", session.token.clone()))
            .dispatch()
            .await
            .status();

        assert_eq!(status, Status::NoContent);

        let response = harness
            .client
            .get(format!(
                "/servers/{}/audit_logs?include_users=true",
                &server.id
            ))
            .header(Header::new("x-session-token", session.token.clone()))
            .dispatch()
            .await
            .into_json::<v0::AuditLogQueryResponse>()
            .await
            .expect("Failed to deserialise audit_logs response");

        let v0::AuditLogQueryResponse::AuditLogsAndUsers {
            audit_logs: entries,
            users,
            members,
        } = response
        else {
            panic!("Response included users when shouldnt")
        };

        assert_eq!(entries.len(), 1);

        let entry = &entries[0];

        assert_eq!(entry.reason.as_deref(), Some("Test Reason"));
        assert_eq!(&entry.server, &server.id);
        assert_eq!(&entry.user, &user.id);
        assert_eq!(
            &entry.action,
            &v0::AuditLogEntryAction::ChannelDelete {
                channel: channel.id().to_string(),
                name: "General".to_string()
            }
        );

        assert_eq!(users.len(), 1);
        assert_eq!(&users[0].id, &user.id);
    }
}
