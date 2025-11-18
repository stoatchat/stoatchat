use revolt_database::AuditLogEntryAction;
use revolt_database::util::permissions::DatabasePermissionQuery;
use revolt_database::{util::reference::Reference, Channel, Database, User};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

use crate::util::audit_log_reason::AuditLogReason;

/// # Create Channel
///
/// Create a new Text or Voice channel.
#[openapi(tag = "Server Information")]
#[post("/<server>/channels", data = "<data>")]
pub async fn create_server_channel(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    server: Reference<'_>,
    data: Json<v0::DataCreateServerChannel>,
) -> Result<Json<v0::Channel>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut server = server.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;

    let channel = Channel::create_server_channel(db, &mut server, data, true)
        .await?;

    AuditLogEntryAction::ChannelCreate { channel: channel.id().to_string() }
        .insert(db, server.id, reason.0, user.id)
        .await;

    Ok(Json(channel.into()))
}
