use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Database, User,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::audit_log_reason::AuditLogReason;

/// # Deletes a webhook
///
/// Deletes a webhook
#[openapi(tag = "Webhooks")]
#[delete("/<webhook_id>")]
pub async fn webhook_delete(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    webhook_id: Reference<'_>,
) -> Result<EmptyResponse> {
    let webhook = webhook_id.as_webhook(db).await?;
    let channel = db.fetch_channel(&webhook.channel_id).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageWebhooks)?;

    webhook.delete(db).await?;

    AuditLogEntryAction::WebhookDelete {
        webhook: webhook.id,
        name: webhook.name,
        channel: webhook.channel_id,
    }
    .insert(
        db,
        channel
            .server()
            .expect("Webhook created on non server channel")
            .to_string(),
        reason.0,
        user.id,
    )
    .await;

    Ok(EmptyResponse)
}
