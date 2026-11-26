use revolt_database::{
    AuditLogEntryAction, Database, User,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
};
use revolt_permissions::{ChannelPermission, calculate_channel_permissions};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::audit_log_reason::AuditLogReason;

/// # Delete Message
///
/// Delete a message you've sent or one you have permission to delete.
#[openapi(tag = "Messaging")]
#[delete("/<target>/messages/<msg>", rank = 2)]
pub async fn delete(
    db: &State<Database>,
    user: User,
    reason: AuditLogReason,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let message = msg.as_message_in_channel(db, target.id).await?;

    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);

    let channel = if message.author != user.id {
        calculate_channel_permissions(&mut query)
            .await
            .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;

        Some(channel)
    } else if message.author == user.id {
        calculate_channel_permissions(&mut query)
            .await
            .throw_if_lacking_channel_permission(ChannelPermission::ViewChannel)?;

        None
    } else {
        None
    };

    message.delete(db).await?;

    if let Some(server) = channel.and_then(|c| c.server().map(|s| s.to_string())) {
        AuditLogEntryAction::MessageDelete {
            author: message.author.clone(),
            channel: message.channel.clone(),
        }
        .insert(
            db,
            server.to_string(),
            reason,
            user.id.clone(),
            Some(message.author),
        )
        .await;
    };

    Ok(EmptyResponse)
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{
        AuditLogQuery, Member, Message, PartialMember, RemovalIntention, Server,
        util::reference::Reference,
    };
    use revolt_models::v0;
    use revolt_permissions::{ChannelPermission, OverrideField};
    use revolt_result::{Error, ErrorType};
    use rocket::http::{Header, Status};

    async fn delete_message(
        harness: &TestHarness,
        token: &str,
        message: &Message,
    ) -> (Status, Option<Error>) {
        let response = harness
            .client
            .delete(format!(
                "/channels/{}/messages/{}",
                message.channel, message.id
            ))
            .header(Header::new("x-session-token", token.to_string()))
            .dispatch()
            .await;

        let status = response.status();
        let error = if status == Status::NoContent {
            None
        } else {
            response.into_json::<Error>().await
        };

        (status, error)
    }

    async fn message_exists(harness: &TestHarness, message: &Message) -> bool {
        Reference::from_unchecked(&message.id)
            .as_message(&harness.db)
            .await
            .is_ok()
    }

    async fn message_delete_audit_entries(harness: &TestHarness, server: &Server) -> usize {
        // audit log inserts are spawned onto a task outside of the database crate's tests
        tokio::time::sleep(Duration::from_millis(250)).await;

        harness
            .db
            .get_server_audit_logs(
                &server.id,
                AuditLogQuery {
                    user: None,
                    target: None,
                    r#type: None,
                    before: None,
                    after: None,
                    limit: 100,
                },
            )
            .await
            .expect("Failed to fetch audit logs")
            .into_iter()
            .filter(|entry| {
                matches!(
                    entry.action,
                    revolt_database::AuditLogEntryAction::MessageDelete { .. }
                )
            })
            .count()
    }

    fn assert_missing_permission(error: Option<Error>, expected: ChannelPermission) {
        match error.expect("Expected an error body").error_type {
            ErrorType::MissingPermission { permission } => {
                assert_eq!(permission, expected.to_string())
            }
            other => panic!("Expected MissingPermission, got {other:?}"),
        }
    }

    #[rocket::async_test]
    async fn author_can_delete_own_message() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, author) = harness.new_user().await;
        let (server, channels) = harness.new_server(&owner).await;
        let (_, _, message) = harness.new_message(&author, &server, channels).await;

        let (status, _) = delete_message(&harness, &session.token, &message).await;

        assert_eq!(status, Status::NoContent);
        assert!(!message_exists(&harness, &message).await);
        assert_eq!(message_delete_audit_entries(&harness, &server).await, 0);
    }

    #[rocket::async_test]
    async fn author_cannot_delete_after_leaving_server() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, author) = harness.new_user().await;
        let (server, channels) = harness.new_server(&owner).await;
        let (_, member, message) = harness.new_message(&author, &server, channels).await;

        member
            .remove(&harness.db, &server, RemovalIntention::Leave, true)
            .await
            .expect("Failed to remove member");

        let (status, error) = delete_message(&harness, &session.token, &message).await;

        assert_eq!(status, Status::Forbidden);
        assert_missing_permission(error, ChannelPermission::ViewChannel);
        assert!(message_exists(&harness, &message).await);
    }

    #[rocket::async_test]
    async fn author_cannot_delete_without_view_channel() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, author) = harness.new_user().await;
        let (server, channels) = harness.new_server(&owner).await;
        let (_, mut member, message) = harness.new_message(&author, &server, channels).await;

        let role = harness
            .new_role(
                &server,
                1,
                Some(OverrideField {
                    a: 0,
                    d: ChannelPermission::ViewChannel as i64,
                }),
            )
            .await;

        member
            .update(
                &harness.db,
                PartialMember {
                    roles: Some(vec![role.id]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to assign role");

        let (status, error) = delete_message(&harness, &session.token, &message).await;

        assert_eq!(status, Status::Forbidden);
        assert_missing_permission(error, ChannelPermission::ViewChannel);
        assert!(message_exists(&harness, &message).await);
    }

    #[rocket::async_test]
    async fn non_author_cannot_delete_without_manage_messages() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, _, author) = harness.new_user().await;
        let (_, session, other) = harness.new_user().await;
        let (server, channels) = harness.new_server(&owner).await;
        let (_, _, message) = harness.new_message(&author, &server, channels).await;

        Member::create(&harness.db, &server, &other, None)
            .await
            .expect("Failed to create member");

        let (status, error) = delete_message(&harness, &session.token, &message).await;

        assert_eq!(status, Status::Forbidden);
        assert_missing_permission(error, ChannelPermission::ManageMessages);
        assert!(message_exists(&harness, &message).await);
    }

    #[rocket::async_test]
    async fn moderator_can_delete_others_message() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, _, author) = harness.new_user().await;
        let (_, session, moderator) = harness.new_user().await;
        let (server, channels) = harness.new_server(&owner).await;
        let (_, _, message) = harness.new_message(&author, &server, channels).await;

        let (mut member, _) = Member::create(&harness.db, &server, &moderator, None)
            .await
            .expect("Failed to create member");

        let role = harness
            .new_role(
                &server,
                1,
                Some(OverrideField {
                    a: ChannelPermission::ManageMessages as i64,
                    d: 0,
                }),
            )
            .await;

        member
            .update(
                &harness.db,
                PartialMember {
                    roles: Some(vec![role.id]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to assign role");

        let (status, _) = delete_message(&harness, &session.token, &message).await;

        assert_eq!(status, Status::NoContent);
        assert!(!message_exists(&harness, &message).await);
        assert_eq!(message_delete_audit_entries(&harness, &server).await, 1);
    }
}
