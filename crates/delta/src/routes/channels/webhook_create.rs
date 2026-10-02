use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    AuditLogEntryAction, Channel, Database, File, User, Webhook,
};
use revolt_models::v0;
use revolt_permissions::{
    calculate_channel_permissions, ChannelPermission, DEFAULT_WEBHOOK_PERMISSIONS,
};
use crate::routes::channels::webhook_fetch_all::fetch_webhooks;
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use ulid::Ulid;
use validator::Validate;

use crate::util::audit_log_reason::AuditLogReason;

/// # Creates a webhook
///
/// Creates a webhook which 3rd party platforms can use to send messages
#[openapi(tag = "Webhooks")]
#[post("/<channel_id>/webhooks", data = "<data>")]
pub async fn create_webhook(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    channel_id: Reference<'_>,
    data: Json<v0::CreateWebhookBody>,
) -> Result<Json<v0::Webhook>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let channel = channel_id.as_channel(db).await?;

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

    if let Some(server_id) = channel.server() {
        AuditLogEntryAction::WebhookCreate {
            webhook: webhook.id.clone(),
            name: webhook.name.clone(),
            channel: webhook.channel_id.clone(),
        }
        .insert(db, server_id.to_string(), reason, user.id, None)
        .await;
    };

    Ok(Json(webhook.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Member, Server};
    use revolt_models::v0;
    use rocket::http::{Header, Status};
    use revolt_database::util::reference::Reference;
    use rocket::State;

    #[rocket::async_test]
    async fn create_webhook_success() {
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
        .expect("Failed to create test server");

        let (_, channels) = Member::create(&harness.db, &server, &user, Some(channels))
            .await
            .expect("Failed to create member");
        let channel = &channels[0];

        let response = harness
            .client
            .post(format!("/channels/{}/webhooks", channel.id()))
            .header(Header::new("x-session-token", session.token.to_string()))
            .json(&v0::CreateWebhookBody {
                name: "Test Webhook".to_string(),
                avatar: None,
            })
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let webhook: v0::Webhook = response.into_json().await.unwrap();
        assert_eq!(webhook.name, "Test Webhook");
        assert_eq!(webhook.channel_id, channel.id());
    }

    #[rocket::async_test]
    async fn create_webhook_unauthorized() {
        let harness = TestHarness::new().await;
        let (_, owner_session, owner_user) = harness.new_user().await;
        let (_, unpriv_session, _) = harness.new_user().await;

        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Test Server".to_string(),
                ..Default::default()
            },
            &owner_user,
            true,
        )
        .await
        .expect("Failed to create test server");

        let (_, channels) = Member::create(&harness.db, &server, &owner_user, Some(channels))
            .await
            .expect("Failed to create member");
        let channel = &channels[0];

        let response = harness
            .client
            .post(format!("/channels/{}/webhooks", channel.id()))
            .header(Header::new("x-session-token", unpriv_session.token.to_string()))
            .json(&v0::CreateWebhookBody {
                name: "Unauthorized Webhook".to_string(),
                avatar: None,
            })
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[tokio::test]
    async fn test_fetch_webhooks_permission_denied() {
        let harness = crate::util::test::TestHarness::new().await;
        let (_, _session, user) = harness.new_user().await;

        let result = fetch_webhooks(
            State::from(&harness.db),
            user,
            Reference::from_unchecked("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_webhooks_success() {
        let harness = crate::util::test::TestHarness::new().await;
        let (_, _session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;
        let channel = harness.new_channel(&server).await;

        let result = fetch_webhooks(
            State::from(&harness.db),
            user,
            Reference::from_unchecked(&channel.id()),
        )
        .await;

        assert!(result.is_ok());
        let Json(webhooks) = result.unwrap();
        assert_eq!(webhooks.len(), 0);
    }
}