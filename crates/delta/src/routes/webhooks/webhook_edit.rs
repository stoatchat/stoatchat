use revolt_database::{
    Database, File, PartialWebhook, User,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
};
use revolt_models::v0::{DataEditWebhook, Webhook};
use revolt_permissions::{ChannelPermission, Override, calculate_channel_permissions};
use revolt_result::{Result, create_error};
use rocket::{State, serde::json::Json};
use validator::Validate;

/// # Edits a webhook
///
/// Edits a webhook
#[openapi(tag = "Webhooks")]
#[patch("/<webhook_id>", data = "<data>")]
pub async fn webhook_edit(
    db: &State<Database>,
    webhook_id: Reference<'_>,
    user: User,
    data: Json<DataEditWebhook>,
) -> Result<Json<Webhook>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut webhook = webhook_id.as_webhook(db).await?;
    let channel = db.fetch_channel(&webhook.channel_id).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let user_permissions = calculate_channel_permissions(&mut query).await;

    user_permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageWebhooks)?;

    if data.name.is_none()
        && data.avatar.is_none()
        && data.permissions.is_none()
        && data.remove.is_empty()
    {
        return Ok(Json(webhook.into()));
    };

    if let Some(new_permissions) = data.permissions {
        let current_value = webhook.permissions;

        user_permissions
            .throw_permission_override(
                Override {
                    allow: current_value,
                    deny: 0,
                },
                &Override {
                    allow: new_permissions,
                    deny: 0,
                },
            )
            .await?;
    }

    let DataEditWebhook {
        name,
        avatar,
        permissions,
        remove,
    } = data;

    let mut partial = PartialWebhook {
        name,
        permissions,
        ..Default::default()
    };

    if let Some(avatar) = avatar {
        let file = File::use_webhook_avatar(db, &avatar, &webhook.id, &webhook.creator_id).await?;
        partial.avatar = Some(file)
    }

    webhook
        .update(db, partial, remove.into_iter().map(|v| v.into()).collect())
        .await?;

    Ok(Json(webhook.into()))
}
