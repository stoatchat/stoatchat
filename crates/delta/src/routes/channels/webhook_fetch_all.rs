use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_models::v0::Webhook;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::{serde::json::Json, State};

/// # Gets all webhooks
///
/// Gets all webhooks inside the channel
#[openapi(tag = "Webhooks")]
#[get("/<channel_id>/webhooks")]
pub async fn fetch_webhooks(
    db: &State<Database>,
    user: User,
    channel_id: Reference<'_>,
) -> Result<Json<Vec<Webhook>>> {
    let channel = channel_id.as_channel(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageWebhooks)?;

    Ok(Json(
        db.fetch_webhooks_for_channel(channel.id())
            .await?
            .into_iter()
            .map(|v| v.into())
            .collect::<Vec<Webhook>>(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use revolt_database::util::reference::Reference;
    use revolt_result::ErrorType;
    use rocket::State;

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

        assert!(matches!(
            result.as_ref().unwrap_err().error_type,
            ErrorType::MissingPermission { .. } | ErrorType::NotFound
        ));
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