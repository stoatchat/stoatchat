use revolt_database::{util::permissions::DatabasePermissionQuery, Database, Message, User};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result, ToRevoltError};
use revolt_search::{ElasticsearchClient, SearchTerms};
use rocket::{serde::json::Json, State};

#[openapi(tag = "Search")]
#[post("/", data = "<data>")]
pub async fn search_messages(
    db: &State<Database>,
    es: &State<ElasticsearchClient>,
    user: User,
    data: Json<v0::DataChannelMessagesSearch>,
) -> Result<Json<v0::BulkMessageResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let data = data.into_inner();

    if (data.server.is_some() && data.channel.is_some())
        || (data.server.is_none() && data.channel.is_none())
    {
        return Err(create_error!(InvalidOperation));
    }

    let (server_id, channels) = if let Some(server_id) = data.server {
        let server = db.fetch_server(&server_id).await?;
        let server_channels = db.fetch_channels(&server.channels).await?;
        let mut channels = Vec::new();

        let query = DatabasePermissionQuery::new(db, &user).server(&server);

        for channel in server_channels {
            let perms = calculate_channel_permissions(&mut query.clone().channel(&channel)).await;

            if perms.has(ChannelPermission::ReadMessageHistory as u64) {
                channels.push(channel.id().to_string())
            }
        }

        (Some(server_id.clone()), channels)
    } else {
        let channel_id = data.channel.unwrap();
        let channel = db.fetch_channel(&channel_id).await?;
        let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
        let perms = calculate_channel_permissions(&mut query).await;

        if !perms.has(ChannelPermission::ReadMessageHistory as u64) {
            return Err(create_error!(InvalidOperation));
        }

        (channel.server().map(ToString::to_string), vec![channel_id])
    };

    let terms = SearchTerms {
        channels,
        filters: data.filters.map(Into::into).unwrap_or_default(),
        offset: data.offset,
        limit: data.limit.map(|limit| limit.min(100)),
        sort: data.sort.map(Into::into),
    };

    let message_ids = es.search(terms).await.to_internal_error()?;

    let mut messages = db.fetch_messages_by_id(&message_ids).await?;

    // Fetching the messages looses the order so resort
    let sort_order = data.sort.unwrap_or_default();
    messages.sort_by(|a, b| match sort_order {
        v0::SortOrder::Asc => a.id.cmp(&b.id),
        v0::SortOrder::Desc => b.id.cmp(&a.id),
    });

    let (users, members) = Message::fetch_users(db, &messages, server_id.as_deref()).await?;

    let response = v0::BulkMessageResponse::MessagesAndUsers {
        messages: messages
            .into_iter()
            .map(|msg| msg.into_model(None, None))
            .collect(),
        users: User::into_mutuals(&user, users).await,
        members: Some(members.into_iter().map(Into::into).collect()),
    };

    Ok(Json(response))
}
