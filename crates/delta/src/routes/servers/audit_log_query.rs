use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntry, AuditLogQuery, Database, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Audit Log Query
///
/// Queries a server's audit logs.
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
    } = options;

    let audit_logs = db
        .get_server_audit_logs(
            &server.id,
            AuditLogQuery {
                user: user_filter,
                r#type,
                before,
                after,
                limit: limit.unwrap_or(50),
            },
        )
        .await?;

    let (users, members) = AuditLogEntry::with_users(db, &server.id, &user, &audit_logs).await?;

    Ok(Json(v0::AuditLogQueryResponse {
        audit_logs: audit_logs.into_iter().map(Into::into).collect(),
        users,
        members,
    }))
}

#[cfg(test)]
mod test {
    use revolt_database::{Member, Server};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    use crate::util::test::TestHarness;

    #[rocket::async_test]
    async fn audit_log_query() {
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
        Member::create(&harness.db, &server, &user, None).await.unwrap();

        let channel = &channels[0];

        let status = harness
            .client
            .patch(format!("/channels/{}", channel.id()))
            .header(Header::new("X-Audit-Log-Reason", "Test Reason 1"))
            .header(Header::new("x-session-token", session.token.clone()))
            .json(&v0::DataEditChannel {
                description: Some("General chat channel.".to_string()),
                name: None,
                owner: None,
                icon: None,
                nsfw: None,
                archived: None,
                voice: None,
                slowmode: None,
                remove: Vec::new(),
            })
            .dispatch()
            .await
            .status();

        assert_eq!(status, Status::Ok);

        let status = harness
            .client
            .patch(format!("/channels/{}", channel.id()))
            .header(Header::new("X-Audit-Log-Reason", "Test Reason 2"))
            .header(Header::new("x-session-token", session.token.clone()))
            .json(&v0::DataEditChannel {
                description: Some("New description.".to_string()),
                name: None,
                owner: None,
                icon: None,
                nsfw: None,
                archived: None,
                voice: None,
                slowmode: None,
                remove: Vec::new(),
            })
            .dispatch()
            .await
            .status();

        assert_eq!(status, Status::Ok);

        let status = harness
            .client
            .delete(format!("/channels/{}", channel.id()))
            .header(Header::new("X-Audit-Log-Reason", "Test Reason 3"))
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

        let v0::AuditLogQueryResponse {
            audit_logs: entries,
            users,
            members,
        } = response;

        assert_eq!(entries.len(), 3);
        assert_eq!(users.len(), 1);
        assert_eq!(members.len(), 1);

        assert_eq!(&users[0].id, &user.id);

        let entry = &entries[0];

        assert_eq!(entry.reason.as_deref(), Some("Test Reason 3"));
        assert_eq!(&entry.server, &server.id);
        assert_eq!(&entry.user, &user.id);
        assert_eq!(
            &entry.action,
            &v0::AuditLogEntryAction::ChannelDelete {
                channel: channel.id().to_string(),
                name: "General".to_string()
            }
        );

        let entry = &entries[1];

        assert_eq!(entry.reason.as_deref(), Some("Test Reason 2"));
        assert_eq!(&entry.server, &server.id);
        assert_eq!(&entry.user, &user.id);
        assert_eq!(
            &entry.action,
            &v0::AuditLogEntryAction::ChannelEdit {
                channel: channel.id().to_string(),
                before: v0::PartialChannel {
                    description: Some("General chat channel.".to_string()),
                    ..Default::default()
                },
                after: v0::PartialChannel {
                    description: Some("New description.".to_string()),
                    ..Default::default()
                }
            }
        );

        let entry = &entries[2];

        assert_eq!(entry.reason.as_deref(), Some("Test Reason 1"));
        assert_eq!(&entry.server, &server.id);
        assert_eq!(&entry.user, &user.id);
        assert_eq!(
            &entry.action,
            &v0::AuditLogEntryAction::ChannelEdit {
                channel: channel.id().to_string(),
                before: v0::PartialChannel {
                    description: None,
                    ..Default::default()
                },
                after: v0::PartialChannel {
                    description: Some("General chat channel.".to_string()),
                    ..Default::default()
                }
            }
        );
    }
}
