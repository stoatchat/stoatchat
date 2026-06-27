use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Channel, Database, User, AMQP,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::State;
use rocket_empty::EmptyResponse;

/// # Add Member to Group
///
/// Adds another user to the group.
#[openapi(tag = "Groups")]
#[put("/<group_id>/recipients/<member_id>")]
pub async fn add_member(
    db: &State<Database>,
    amqp: &State<AMQP>,
    user: User,
    group_id: Reference<'_>,
    member_id: Reference<'_>,
) -> Result<EmptyResponse> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let mut channel = group_id.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::InviteOthers)?;

    match &channel {
        Channel::Group { .. } => {
            // TODO: use permissions here? interesting if users could block new group invites
            let member = member_id.as_user(db).await?;
            if !user.is_friends_with(&member.id) {
                return Err(create_error!(NotFriends));
            }

            channel
                .add_user_to_group(db, amqp, &member, &user.id)
                .await
                .map(|_| EmptyResponse)
        }
        _ => Err(create_error!(InvalidOperation)),
    }
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{events::client::EventV1, Channel, RelationshipStatus};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn success_add_member() {
        let mut harness = TestHarness::new().await;
        let (_, session, mut user) = harness.new_user().await;
        let (_, _, mut other_user) = harness.new_user().await;

        #[allow(clippy::disallowed_methods)]
        user.apply_relationship(
            &harness.db,
            &mut other_user,
            RelationshipStatus::Friend,
            RelationshipStatus::Friend,
        )
        .await
        .unwrap();

        let group = Channel::create_group(
            &harness.db,
            v0::DataCreateGroup {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            user.id.to_string(),
        )
        .await
        .unwrap();

        let response = harness
            .client
            .put(format!(
                "/channels/{}/recipients/{}",
                group.id(),
                other_user.id
            ))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        drop(response);

        harness
            .wait_for_event(&format!("{}!", other_user.id), |event| match event {
                EventV1::ChannelCreate(channel) => channel.id() == group.id(),
                _ => false,
            })
            .await;

        let event = harness
            .wait_for_event(group.id(), |event| match event {
                EventV1::ChannelGroupJoin { id, .. } => id == group.id(),
                _ => false,
            })
            .await;

        match event {
            EventV1::ChannelGroupJoin { user, .. } => assert_eq!(user, other_user.id),
            _ => unreachable!(),
        };

        let message = harness.wait_for_message(group.id()).await;

        assert_eq!(
            message.system,
            Some(v0::SystemMessage::UserAdded {
                id: other_user.id.to_string(),
                by: user.id.to_string()
            })
        );
    }

    #[rocket::async_test]
    async fn fail_add_non_friend() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (_, _, other_user) = harness.new_user().await;

        let group = Channel::create_group(
            &harness.db,
            v0::DataCreateGroup {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            user.id.to_string(),
        )
        .await
        .unwrap();

        let response = harness
            .client
            .put(format!(
                "/channels/{}/recipients/{}",
                group.id(),
                other_user.id
            ))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn fail_add_already_in_group() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let group = Channel::create_group(
            &harness.db,
            v0::DataCreateGroup {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            user.id.to_string(),
        )
        .await
        .unwrap();

        let response = harness
            .client
            .put(format!("/channels/{}/recipients/{}", group.id(), user.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Conflict);
    }
}
