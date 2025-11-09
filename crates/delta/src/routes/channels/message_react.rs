use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Add Reaction to Message
///
/// React to a given message.
#[openapi(tag = "Interactions")]
#[put("/<target>/messages/<msg>/reactions/<emoji>")]
pub async fn react_message(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
    emoji: Reference<'_>,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::React)?;

    // Fetch relevant message
    let message = msg.as_message_in_channel(db, channel.id()).await?;

    // Add the reaction
    message
        .add_reaction(db, &user, emoji.id)
        .await
        .map(|_| EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{events::client::EventV1, Emoji, EmojiParent};
    use rocket::http::Status;
    use url::form_urlencoded;

    fn new_emoji() -> (Emoji, String) {
        let emoji = Emoji {
            id: "😀".to_string(),
            parent: EmojiParent::Detached,
            creator_id: format!("{}!", TestHarness::rand_string()),
            name: "smile".to_string(),
            animated: false,
            nsfw: false,
        };

        let encoded_emoji: String = form_urlencoded::byte_serialize(emoji.id.as_bytes()).collect();

        (emoji, encoded_emoji)
    }

    #[rocket::async_test]
    async fn success_react_message() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, message) = harness.new_message(&user, &server, channels).await;

        let (emoji, encoded_emoji) = new_emoji();

        let put_response = TestHarness::with_session(
            session,
            harness.client.put(format!(
                "/channels/{}/messages/{}/reactions/{}",
                channel.id(),
                message.id,
                encoded_emoji
            )),
        )
        .await;

        assert_eq!(put_response.status(), Status::NoContent);
        drop(put_response);

        let event = harness
            .wait_for_event(channel.id(), |event| match event {
                EventV1::MessageReact { id, .. } => *id == message.id,
                _ => false,
            })
            .await;

        match event {
            EventV1::MessageReact {
                id,
                channel_id,
                user_id,
                emoji_id,
            } => {
                assert_eq!(id, message.id);
                assert_eq!(channel_id, channel.id());
                assert_eq!(user_id, user.id);
                assert_eq!(emoji_id, emoji.id);
            }
            _ => unreachable!(),
        };
    }

    #[rocket::async_test]
    async fn fail_not_found_react_message() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, _) = harness.new_message(&user, &server, channels).await;

        let (_, encoded_emoji) = new_emoji();

        let put_response = TestHarness::with_session(
            session,
            harness.client.put(format!(
                "/channels/{}/messages/{}/reactions/{}",
                channel.id(),
                TestHarness::rand_string(),
                encoded_emoji
            )),
        )
        .await;

        assert_eq!(put_response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn fail_forbidden_react_message() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, message) = harness.new_message(&user, &server, channels).await;

        let (_, session2, _) = harness.new_user().await;

        let (_, encoded_emoji) = new_emoji();

        let put_response = TestHarness::with_session(
            session2,
            harness.client.put(format!(
                "/channels/{}/messages/{}/reactions/{}",
                channel.id(),
                message.id,
                encoded_emoji
            )),
        )
        .await;

        assert_eq!(put_response.status(), Status::Forbidden);
    }
}
