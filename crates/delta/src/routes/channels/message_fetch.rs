use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Fetch Message
///
/// Retrieves a message by its id.
#[openapi(tag = "Messaging")]
#[get("/<target>/messages/<msg>")]
pub async fn fetch(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<Json<v0::Message>> {
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ViewChannel)?;

    let message = msg.as_message(db).await?;
    if message.channel != channel.id() {
        return Err(create_error!(NotFound));
    }

    Ok(Json(message.into_model(None, None)))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Channel, Message};
    use revolt_models::v0::DataCreateGroup;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn success_fetch_message() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, message) = harness.new_message(&user, &server, channels).await;

        let fetch_response = TestHarness::with_session(
            session,
            harness.client.get(format!(
                "/channels/{}/messages/{}",
                channel.id(),
                message.id
            )),
        )
        .await;

        assert_eq!(fetch_response.status(), Status::Ok);
        let fetched_message: Message = fetch_response.into_json().await.expect("`Message`");

        match fetched_message {
            Message {
                id,
                channel,
                author,
                ..
            } => {
                assert_eq!(id, message.id);
                assert_eq!(channel, message.channel);
                assert_eq!(author, message.author);
            }
        }
    }

    #[rocket::async_test]
    async fn fail_not_found_fetch_message() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let group = Channel::create_group(
            &harness.db,
            DataCreateGroup {
                ..Default::default()
            },
            user.id.clone(),
        )
        .await
        .expect("`Channel`");

        let fetch_response = TestHarness::with_session(
            session,
            harness.client.get(format!(
                "/channels/{}/messages/{}",
                group.id(),
                TestHarness::rand_string()
            )),
        )
        .await;

        assert_eq!(fetch_response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn fail_forbidden_fetch_message() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, message) = harness.new_message(&user, &server, channels).await;

        let (_, session2, _) = harness.new_user().await;

        let fetch_response = TestHarness::with_session(
            session2,
            harness.client.get(format!(
                "/channels/{}/messages/{}",
                channel.id(),
                message.id
            )),
        )
        .await;

        assert_eq!(fetch_response.status(), Status::Forbidden);
    }
}
