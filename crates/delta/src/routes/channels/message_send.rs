use chrono::{Duration, Utc};
use revolt_database::util::permissions::DatabasePermissionQuery;
use revolt_database::{
    util::idempotency::IdempotencyKey, util::reference::Reference, Database, User,
};
use revolt_database::{Interactions, Message, AMQP};
use revolt_models::v0;
use revolt_permissions::PermissionQuery;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

/// # Send Message
///
/// Sends a message to the given channel.
#[openapi(tag = "Messaging")]
#[post("/<target>/messages", data = "<data>")]
pub async fn message_send(
    db: &State<Database>,
    amqp: &State<AMQP>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::DataMessageSend>,
    idempotency: IdempotencyKey,
) -> Result<Json<v0::Message>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    // Ensure we have permissions to send a message
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;
    permissions.throw_if_lacking_channel_permission(ChannelPermission::SendMessage)?;

    // Verify permissions for masquerade
    if let Some(masq) = &data.masquerade {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::Masquerade)?;

        if masq.colour.is_some() {
            permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageRole)?;
        }
    }

    // Check permissions for embeds
    if data.embeds.as_ref().is_some_and(|v| !v.is_empty()) {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::SendEmbeds)?;
    }

    // Check permissions for files
    if data.attachments.as_ref().is_some_and(|v| !v.is_empty()) {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::UploadFiles)?;
    }

    // Ensure interactions information is correct
    if let Some(interactions) = &data.interactions {
        let interactions: Interactions = interactions.clone().into();
        interactions.validate(db, &permissions).await?;
    }

    // Disallow mentions for new users (TRUST-0: <12 hours age) in public servers
    let allow_mentions = if let Some(server) = query.server_ref() {
        if server.discoverable {
            (Utc::now() - ulid::Ulid::from_string(&user.id).unwrap().datetime())
                >= Duration::hours(12)
        } else {
            true
        }
    } else {
        true
    };

    // Create the message
    let author: v0::User = user.clone().into(db, Some(&user)).await;

    // Make sure we have server member (edge case if server owner)
    query.are_we_a_member().await;

    // Create model user / members
    let model_user = user
        .clone()
        .into_known_static(revolt_presence::is_online(&user.id).await)
        .await;

    let model_member: Option<v0::Member> = query
        .member_ref()
        .as_ref()
        .map(|member| member.clone().into_owned().into());

    Ok(Json(
        Message::create_from_api(
            db,
            Some(amqp),
            channel,
            data,
            v0::MessageAuthor::User(&author),
            Some(model_user.clone()),
            model_member.clone(),
            user.limits().await,
            idempotency,
            permissions.has_channel_permission(ChannelPermission::SendEmbeds),
            allow_mentions,
        )
        .await?
        .into_model(Some(model_user), model_member),
    ))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{
        util::{idempotency::IdempotencyKey, reference::Reference},
        Channel, Member, Message, MessageFlagsValue, PartialChannel, PartialMember, Role, Server,
    };
    use revolt_models::v0::{self, DataCreateServerChannel, MessageFlags};
    use revolt_permissions::{ChannelPermission, OverrideField};
    use revolt_result::ErrorType;

    #[rocket::async_test]
    async fn message_mention_constraints() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (_, _, second_user) = harness.new_user().await;

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

        let server_mut: &mut Server = &mut server.clone();
        let mut locked_channel = Channel::create_server_channel(
            &harness.db,
            server_mut,
            DataCreateServerChannel {
                channel_type: v0::LegacyServerChannelType::Text,
                name: "Hidden Channel".to_string(),
                description: None,
                nsfw: Some(false),
                voice: None
            },
            true,
        )
        .await
        .expect("Failed to make new channel");

        let role = Role {
            name: "Show Hidden Channel".to_string(),
            permissions: OverrideField { a: 0, d: 0 },
            colour: None,
            hoist: false,
            rank: 5,
        };

        let role_id = role
            .create(&harness.db, &server.id)
            .await
            .expect("Failed to create the role");

        let mut overrides = HashMap::new();
        overrides.insert(
            role_id.clone(),
            OverrideField {
                a: (ChannelPermission::ViewChannel) as i64,
                d: 0,
            },
        );

        let partial = PartialChannel {
            name: None,
            owner: None,
            description: None,
            icon: None,
            nsfw: None,
            active: None,
            permissions: None,
            role_permissions: Some(overrides),
            default_permissions: Some(OverrideField {
                a: 0,
                d: ChannelPermission::ViewChannel as i64,
            }),
            last_message_id: None,
            voice: None,
        };
        locked_channel
            .update(&harness.db, partial, vec![])
            .await
            .expect("Failed to update the channel permissions for special role");

        Member::create(&harness.db, &server, &user, Some(channels.clone()))
            .await
            .expect("Failed to create member");
        let member = Reference::from_unchecked(&user.id)
            .as_member(&harness.db, &server.id)
            .await
            .expect("Failed to get member");

        // Second user is not part of the server
        let message = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            locked_channel.clone(),
            v0::DataMessageSend {
                content: Some(format!("<@{}>", second_user.id)),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("0".to_string()),
            false,
            true,
        )
        .await
        .expect("Failed to create message");

        // The mention should not go through here
        assert!(
            message.mentions.is_none() || message.mentions.unwrap().is_empty(),
            "Mention failed to be scrubbed when the user is not part of the server"
        );

        Member::create(&harness.db, &server, &second_user, Some(channels.clone()))
            .await
            .expect("Failed to create second member");
        let mut second_member = Reference::from_unchecked(&second_user.id)
            .as_member(&harness.db, &server.id)
            .await
            .expect("Failed to get second member");

        // Second user cannot see the channel
        let message = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            locked_channel.clone(),
            v0::DataMessageSend {
                content: Some(format!("<@{}>", second_user.id)),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("1".to_string()),
            false,
            true,
        )
        .await
        .expect("Failed to create message");

        // The mention should not go through here
        assert!(
            message.mentions.is_none() || message.mentions.unwrap().is_empty(),
            "Mention failed to be scrubbed when the user cannot see the channel"
        );

        let second_member_roles = vec![role_id.clone()];
        let partial = PartialMember {
            id: None,
            joined_at: None,
            nickname: None,
            avatar: None,
            timeout: None,
            roles: Some(second_member_roles),
            can_publish: None,
            can_receive: None
        };
        second_member
            .update(&harness.db, partial, vec![])
            .await
            .expect("Failed to update the second user's roles");

        // This time the mention SHOULD go through
        let message = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            locked_channel.clone(),
            v0::DataMessageSend {
                content: Some(format!("<@{}>", second_user.id)),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("2".to_string()),
            false,
            true,
        )
        .await
        .expect("Failed to create message");

        // The mention SHOULD go through here
        assert!(
            message.mentions.is_some() && !message.mentions.unwrap().is_empty(),
            "Mention was scrubbed when the user can see the channel"
        );
    }

    #[rocket::async_test]
    async fn message_reply() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let channel = harness.new_channel(&server).await;
        let (_, member, message) = harness.new_message(&user, &server, channels).await;

        // Send a message with a reply
        // Should succeed
        let message_with_reply = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some("Message with reply".to_string()),
                nonce: None,
                attachments: None,
                replies: Some(vec![v0::ReplyIntent {
                    id: message.id.clone(),
                    mention: false,
                    fail_if_not_exists: Some(true),
                }]),
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("1".to_string()),
            false,
            false,
        )
        .await
        .expect("Failed to create message with reply");

        assert!(
            message_with_reply.replies.is_some(),
            "Message replies is None",
        );

        let replies = message_with_reply.replies.unwrap();

        assert!(!replies.is_empty(), "Message replies is empty",);

        assert_eq!(replies[0], message.id, "Message reply ID does not match",);

        // Delete the message
        message
            .clone()
            .delete(&harness.db)
            .await
            .expect("Failed to delete message");

        // Attempt to create messages with a reply to a deleted message

        // fail_if_not_exists is set to false
        // Should send the message without a reply
        let message_with_missing_reply = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some("Message with missing reply".to_string()),
                nonce: None,
                attachments: None,
                replies: Some(vec![v0::ReplyIntent {
                    id: message.id.clone(),
                    mention: false,
                    fail_if_not_exists: Some(false),
                }]),
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("3".to_string()),
            false,
            false,
        )
        .await
        .expect("Failed to create message with missing reply");

        assert!(
            message_with_missing_reply.replies.is_none()
                || message_with_missing_reply.replies.unwrap().is_empty(),
            "Message replies exist when they shouldn't",
        );

        // fail_if_not_exists is set to true
        // Should fail to send the message
        Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some("Message with missing reply".to_string()),
                nonce: None,
                attachments: None,
                replies: Some(vec![v0::ReplyIntent {
                    id: message.id.clone(),
                    mention: false,
                    fail_if_not_exists: Some(true),
                }]),
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("4".to_string()),
            false,
            false,
        )
        .await
        .expect_err("Created message with missing reply and true fail");

        // fail_if_not_exists is not set
        // Should fail to send the message
        Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some("Message with missing reply".to_string()),
                nonce: None,
                attachments: None,
                replies: Some(vec![v0::ReplyIntent {
                    id: message.id.clone(),
                    mention: false,
                    fail_if_not_exists: None,
                }]),
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.clone().into()),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("4".to_string()),
            false,
            false,
        )
        .await
        .expect_err("Created message with missing reply and none fail");
    }

    #[rocket::async_test]
    async fn mass_mentions_test() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;
        let (_, _, other_user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;
        let channel = harness.new_channel(&server).await;
        let (role_id, _role) = harness
            .new_role(
                &server,
                1,
                Some(OverrideField {
                    a: (ChannelPermission::MentionEveryone as i64)
                        | (ChannelPermission::MentionRoles as i64),
                    d: 0,
                }),
            )
            .await;
        let (mut other_member, _) = Member::create(&harness.db, &server, &other_user, None)
            .await
            .expect("Failed to add test member");

        // Send a message with an everyone and role mention.
        // Should fail
        let bad_message_with_mentions = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some(format!("Mentioning @everyone and role <%{}>", &role_id)),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(
                &other_user
                    .clone()
                    .into(&harness.db, Some(&other_user))
                    .await,
            ),
            Some(
                other_user
                    .clone()
                    .into(&harness.db, Some(&other_user))
                    .await,
            ),
            Some(other_member.clone().into()),
            other_user.limits().await,
            IdempotencyKey::unchecked_from_string("1".to_string()),
            false,
            true,
        )
        .await
        .expect_err("Should not have created message with everyone and role pings");

        assert!(
            matches!(
                bad_message_with_mentions.error_type,
                ErrorType::MissingPermission { .. }
            ),
            "Intentional permissions error did not return MissingPermission"
        );

        // Send a mass mention inside a codeblock.
        // Should be undetected and therefor pass
        let message_with_codeblock = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some(format!("Mentioning `@everyone` and role `<%{}>`", &role_id)),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(
                &other_user
                    .clone()
                    .into(&harness.db, Some(&other_user))
                    .await,
            ),
            Some(
                other_user
                    .clone()
                    .into(&harness.db, Some(&other_user))
                    .await,
            ),
            Some(other_member.clone().into()),
            other_user.limits().await,
            IdempotencyKey::unchecked_from_string("1".to_string()),
            false,
            true,
        )
        .await
        .expect("Failed to create message with everyone and role pings in codeblocks");

        assert!(
            message_with_codeblock.flags.is_none()
                || !MessageFlagsValue(message_with_codeblock.flags.unwrap())
                    .has(MessageFlags::MentionsEveryone),
            "Message flags mentions everyone when inside codeblock",
        );

        assert!(
            message_with_codeblock.role_mentions.is_none(),
            "Role mentions detected when inside codeblock"
        );

        other_member.roles.push(role_id.clone());
        harness
            .db
            .update_member(
                &other_member.id,
                &PartialMember {
                    avatar: None,
                    id: None,
                    joined_at: None,
                    nickname: None,
                    roles: Some(vec![role_id.clone()]),
                    timeout: None,
                    can_publish: None,
                    can_receive: None
                },
                vec![],
            )
            .await
            .expect("Failed to add role to user");

        // Send a message with an everyone and role mention.
        // Should succeed
        let message_with_mentions = Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some(format!("Mentioning @everyone and role <%{}>", &role_id)),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(
                &other_user
                    .clone()
                    .into(&harness.db, Some(&other_user))
                    .await,
            ),
            Some(
                other_user
                    .clone()
                    .into(&harness.db, Some(&other_user))
                    .await,
            ),
            Some(other_member.clone().into()),
            other_user.limits().await,
            IdempotencyKey::unchecked_from_string("1".to_string()),
            false,
            true,
        )
        .await
        .expect("Failed to create message with everyone and role pings");

        assert!(
            message_with_mentions.flags.is_some(),
            "Message flags is None",
        );

        assert!(
            MessageFlagsValue(message_with_mentions.flags.unwrap())
                .has(MessageFlags::MentionsEveryone),
            "Message flags does not mention everyone. Flag value is {}",
            message_with_mentions.flags.unwrap()
        );

        assert!(
            message_with_mentions.role_mentions.is_some(),
            "Message has no role mentions"
        );
    }
}
