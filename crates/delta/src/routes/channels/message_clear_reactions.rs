use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, PartialMessage, User,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Remove All Reactions from Message
///
/// Remove your own, someone else's or all of a given reaction.
///
/// Requires `ManageMessages` permission.
#[openapi(tag = "Interactions")]
#[delete("/<target>/messages/<msg>/reactions")]
pub async fn clear_reactions(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    msg: Reference<'_>,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;

    // Fetch relevant message
    let mut message = msg.as_message_in_channel(db, channel.id()).await?;

    // Clear reactions
    message
        .update(
            db,
            PartialMessage {
                reactions: Some(Default::default()),
                ..Default::default()
            },
            vec![]
        )
        .await
        .map(|_| EmptyResponse)
}
