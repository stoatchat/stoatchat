use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Channel, Database, PartialMessage, User, AMQP,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Marks a message as the solution to its forum post
///
/// Marks a reply as the accepted solution to the forum post it replies to.
/// Only valid in a `ForumChannel` with `solution_enabled`, and only on a
/// message that is itself a reply (not a post's root message).
#[openapi(tag = "Messaging")]
#[post("/<target>/messages/<msg>/solution")]
pub async fn message_mark_solution(
    db: &State<Database>,
    _amqp: &State<AMQP>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;

    let solution_enabled = match &channel {
        Channel::ForumChannel {
            solution_enabled, ..
        } => *solution_enabled,
        _ => false,
    };

    if !solution_enabled {
        return Err(create_error!(InvalidOperation));
    }

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;

    let mut message = msg.as_message_in_channel(db, channel.id()).await?;

    // Only a reply (not a post's own root message) can be the solution to a post.
    if message.replies.as_ref().is_none_or(|r| r.is_empty()) {
        return Err(create_error!(InvalidOperation));
    }

    if message.forum_solution.unwrap_or_default() {
        return Err(create_error!(InvalidOperation));
    }

    message
        .update(
            db,
            PartialMessage {
                forum_solution: Some(true),
                ..Default::default()
            },
            vec![],
        )
        .await?;

    Ok(EmptyResponse)
}
