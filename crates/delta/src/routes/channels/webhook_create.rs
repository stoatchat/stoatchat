use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Channel, Database, File, User, Webhook,
};
use revolt_models::v0;
use revolt_permissions::{
    calculate_channel_permissions, ChannelPermission, DEFAULT_WEBHOOK_PERMISSIONS,
};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use ulid::Ulid;
use validator::Validate;

use crate::util::audit_log_reason::AuditLogReason;

/// # Creates a webhook
///
/// Creates a webhook which 3rd party platforms can use to send messages
#[openapi(tag = "Webhooks")]
#[post("/<target>/webhooks", data = "<data>")]
pub async fn create_webhook(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    data: Json<v0::CreateWebhookBody>,
) -> Result<Json<v0::Webhook>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let channel = target.as_channel(db).await?;

    if !matches!(channel, Channel::TextChannel { .. } | Channel::Group { .. }) {
        return Err(create_error!(InvalidOperation));
    }

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageWebhooks)?;

    let webhook_id = Ulid::new().to_string();

    let avatar = match &data.avatar {
        Some(id) => Some(File::use_webhook_avatar(db, id, &webhook_id, &user.id).await?),
        None => None,
    };

    let webhook = Webhook {
        id: webhook_id,
        name: data.name,
        avatar,
        creator_id: user.id.clone(),
        channel_id: channel.id().to_string(),
        permissions: *DEFAULT_WEBHOOK_PERMISSIONS,
        token: Some(nanoid::nanoid!(64)),
    };

    webhook.create(db).await?;

    if let Some(server) = query.server_ref() {
        AuditLogEntryAction::WebhookCreate {
            webhook: webhook.id.clone(),
            name: webhook.name.clone(),
            channel: webhook.channel_id.clone(),
        }
        .insert(db, server.id.clone(), reason.0, user.id)
        .await;
    };

    Ok(Json(webhook.into()))
}
