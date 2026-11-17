use revolt_config::config;
use revolt_database::{
    util::{idempotency::IdempotencyKey, reference::Reference},
    Database, Message, AMQP,
};
use revolt_models::v0;
use revolt_permissions::{ChannelPermission, PermissionValue};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

use validator::Validate;

/// # Executes a webhook
///
/// Executes a webhook and sends a message
#[openapi(tag = "Webhooks")]
#[post("/<webhook_id>/<token>", data = "<data>")]
pub async fn webhook_execute(
    db: &State<Database>,
    amqp: &State<AMQP>,
    webhook_id: Reference<'_>,
    token: String,
    data: Json<v0::DataMessageSend>,
    idempotency: IdempotencyKey,
) -> Result<Json<v0::Message>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let webhook = webhook_id.as_webhook(db).await?;
    webhook.assert_token(&token)?;

    let permissions: PermissionValue = webhook.permissions.into();
    permissions.throw_if_lacking_channel_permission(ChannelPermission::SendMessage)?;

    if data.attachments.as_ref().is_some_and(|v| !v.is_empty()) {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::UploadFiles)?;
    }

    if data.embeds.as_ref().is_some_and(|v| !v.is_empty()) {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::SendEmbeds)?;
    }

    if data.masquerade.is_some() {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::Masquerade)?;
    }

    if data.interactions.is_some() {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::React)?;
    }

    let channel = db.fetch_channel(&webhook.channel_id).await?;

    Ok(Json(
        Message::create_from_api(
            db,
            Some(amqp),
            channel,
            data,
            v0::MessageAuthor::Webhook(&webhook.into()),
            None,
            None,
            config().await.features.limits.default,
            idempotency,
            true,
            true,
        )
        .await?
        .into_model(None, None),
    ))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::Webhook;
    use revolt_models::v0;
    use revolt_permissions::{ChannelPermission, DEFAULT_WEBHOOK_PERMISSIONS};
    use rocket::http::{ContentType, Status};

    async fn execute(harness: &TestHarness, webhook: &Webhook, content: String) -> Status {
        harness
            .client
            .post(format!(
                "/webhooks/{}/{}",
                webhook.id,
                webhook.token.as_ref().unwrap()
            ))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataMessageSend {
                    content: Some(content),
                    nonce: None,
                    attachments: None,
                    replies: None,
                    embeds: None,
                    masquerade: None,
                    interactions: None,
                    flags: None,
                })
                .to_string(),
            )
            .dispatch()
            .await
            .status()
    }

    #[rocket::async_test]
    async fn webhook_mass_mention_permissions() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;
        let channel = harness.new_channel(&server).await;
        let role = harness.new_role(&server, 1, None).await;

        let mut webhook = Webhook {
            id: ulid::Ulid::new().to_string(),
            name: "Test Webhook".to_string(),
            creator_id: user.id.clone(),
            channel_id: channel.id().to_string(),
            permissions: *DEFAULT_WEBHOOK_PERMISSIONS,
            token: Some(TestHarness::rand_string()),
            ..Default::default()
        };
        webhook.create(&harness.db).await.unwrap();

        // Without mention permissions, mass mentions must be rejected
        assert_eq!(
            execute(&harness, &webhook, "@everyone".to_string()).await,
            Status::Forbidden
        );
        assert_eq!(
            execute(&harness, &webhook, "@online".to_string()).await,
            Status::Forbidden
        );
        assert_eq!(
            execute(&harness, &webhook, format!("<%{}>", role.id)).await,
            Status::Forbidden
        );

        // Ordinary messages still go through
        assert_eq!(
            execute(&harness, &webhook, "hello".to_string()).await,
            Status::Ok
        );

        // Granting the permissions allows mass mentions
        webhook.permissions |= ChannelPermission::MentionEveryone as u64
            | ChannelPermission::MentionRoles as u64;
        harness.db.delete_webhook(&webhook.id).await.unwrap();
        webhook.create(&harness.db).await.unwrap();

        assert_eq!(
            execute(&harness, &webhook, "@everyone".to_string()).await,
            Status::Ok
        );
        assert_eq!(
            execute(&harness, &webhook, format!("<%{}>", role.id)).await,
            Status::Ok
        );
    }
}
