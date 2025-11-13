use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Message, MessageFilter, MessageQuery, MessageTimePeriod, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Search for Messages
///
/// This route searches for messages within the given parameters.
#[openapi(tag = "Messaging")]
#[post("/<target>/search", data = "<options>")]
pub async fn search(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    options: Json<v0::DataMessageSearch>,
) -> Result<Json<v0::BulkMessageResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let options = options.into_inner();
    options.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    if options.query.is_some() && options.pinned.is_some() {
        return Err(create_error!(InvalidOperation));
    }

    let channel = target.as_channel(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ReadMessageHistory)?;

    let v0::DataMessageSearch {
        query,
        pinned,
        limit,
        before,
        after,
        sort,
        include_users,
    } = options;

    Message::fetch_with_users(
        db,
        MessageQuery {
            filter: MessageFilter {
                channel: Some(channel.id().to_string()),
                query,
                pinned,
                ..Default::default()
            },
            time_period: MessageTimePeriod::Absolute {
                before,
                after,
                sort: Some(sort),
            },
            limit,
        },
        &user,
        include_users,
        channel.server(),
    )
    .await
    .map(Json)
}
