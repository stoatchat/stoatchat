use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Message, MessageFilter, MessageQuery, MessageTimePeriod, User,
};
use revolt_models::v0::{self, MessageSort};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Fetch Messages
///
/// Fetch multiple messages.
#[openapi(tag = "Messaging")]
#[get("/<target>/messages?<options..>")]
pub async fn query(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    options: v0::OptionsQueryMessages,
) -> Result<Json<v0::BulkMessageResponse>> {
    options.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    if let Some(MessageSort::Relevance) = options.sort {
        return Err(create_error!(InvalidOperation));
    }

    let channel = target.as_channel(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ReadMessageHistory)?;

    let v0::OptionsQueryMessages {
        limit,
        before,
        after,
        sort,
        nearby,
        include_users,
    } = options;

    Message::fetch_with_users(
        db,
        MessageQuery {
            filter: MessageFilter {
                channel: Some(channel.id().to_string()),
                ..Default::default()
            },
            time_period: if let Some(nearby) = nearby {
                MessageTimePeriod::Relative { nearby }
            } else {
                MessageTimePeriod::Absolute {
                    before,
                    after,
                    sort,
                }
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
