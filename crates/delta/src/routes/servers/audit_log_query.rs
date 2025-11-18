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
