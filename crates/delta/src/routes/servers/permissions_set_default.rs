use revolt_database::{
    AuditLogEntryAction, Database, PartialServer, User,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    voice::{VoiceClient, sync_voice_permissions},
};
use revolt_models::v0;
use revolt_permissions::{
    ChannelPermission, DataPermissionsValue, Override, calculate_server_permissions,
};
use revolt_result::Result;
use rocket::{State, serde::json::Json};

use crate::util::audit_log_reason::AuditLogReason;

/// # Set Default Permission
///
/// Sets permissions for the default role in this server.
#[openapi(tag = "Server Permissions")]
#[put("/<target>/permissions/default", data = "<data>", rank = 1)]
pub async fn set_default_server_permissions(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    data: Json<DataPermissionsValue>,
) -> Result<Json<v0::Server>> {
    let data = data.into_inner();

    let mut server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    let permissions = calculate_server_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    // Ensure we have permissions to grant these permissions forwards
    permissions
        .throw_permission_override(
            None,
            &Override {
                allow: data.permissions,
                deny: 0,
            },
        )
        .await?;

    let partial = PartialServer {
        default_permissions: Some(data.permissions as i64),
        ..Default::default()
    };

    let before = server.generate_diff(&partial, &[]);

    server.update(db, partial.clone(), vec![]).await?;

    AuditLogEntryAction::ServerEdit {
        before,
        after: partial,
    }
    .insert(db, server.id.clone(), reason, user.id, None)
    .await;

    let channels = db.fetch_channels(&server.channels).await?;

    for channel in &channels {
        sync_voice_permissions(db, voice_client, &channel, Some(&server), None).await?;
    }

    Ok(Json(
        server
            .into(
                &db,
                &channels.into_iter().map(Into::into).collect::<Vec<_>>(),
            )
            .await,
    ))
}
