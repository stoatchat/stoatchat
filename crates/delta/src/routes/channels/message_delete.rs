use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Delete Message
///
/// Delete a message you've sent or one you have permission to delete.
#[openapi(tag = "Messaging")]
#[delete("/<target>/messages/<msg>", rank = 2)]
pub async fn delete(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let message = msg.as_message_in_channel(db, target.id).await?;

    if message.author != user.id {
        let channel = target.as_channel(db).await?;
        let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
        calculate_channel_permissions(&mut query)
            .await
            .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;
    }

    message.delete(db).await.map(|_| EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::Status;

    #[rocket::async_test]
    async fn success_message_delete() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, message) = harness.new_message(&user, &server, channels).await;

        let delete_response = TestHarness::with_session(
            session,
            harness.client.delete(format!(
                "/channels/{}/messages/{}",
                channel.id(),
                message.id
            )),
        )
        .await;

        assert_eq!(delete_response.status(), Status::NoContent);
    }

    #[rocket::async_test]
    async fn fail_not_found_message_delete() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, _) = harness.new_message(&user, &server, channels).await;

        let delete_response = TestHarness::with_session(
            session,
            harness.client.delete(format!(
                "/channels/{}/messages/{}",
                channel.id(),
                TestHarness::rand_string()
            )),
        )
        .await;

        assert_eq!(delete_response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn fail_forbidden_message_delete() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let (channel, _, message) = harness.new_message(&user, &server, channels).await;

        let (_, session2, _) = harness.new_user().await;

        let delete_response = TestHarness::with_session(
            session2,
            harness.client.delete(format!(
                "/channels/{}/messages/{}",
                channel.id(),
                message.id
            )),
        )
        .await;

        assert_eq!(delete_response.status(), Status::Forbidden);
    }
}
