use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Channel, Database, FieldsMessage, PartialMessage, User, AMQP,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Unmarks a message as the solution to its forum post
#[openapi(tag = "Messaging")]
#[delete("/<target>/messages/<msg>/solution")]
pub async fn message_unmark_solution(
    db: &State<Database>,
    _amqp: &State<AMQP>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;

    if !matches!(channel, Channel::ForumChannel { .. }) {
        return Err(create_error!(InvalidOperation));
    }

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;

    let mut message = msg.as_message_in_channel(db, channel.id()).await?;

    if !message.forum_solution.unwrap_or_default() {
        return Err(create_error!(InvalidOperation));
    }

    message
        .update(
            db,
            PartialMessage::default(),
            vec![FieldsMessage::ForumSolution],
        )
        .await?;

    Ok(EmptyResponse)
}
