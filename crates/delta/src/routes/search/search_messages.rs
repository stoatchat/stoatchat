use std::collections::HashSet;

use revolt_database::{util::permissions::DatabasePermissionQuery, Database, User};
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

    let mut messages = db
        .fetch_messages_by_id(&message_ids)
        .await?
        .into_iter()
        .map(|msg| msg.into_model(None, None))
        .collect::<Vec<_>>();

    // Fetching the messages looses the order so resort
    let sort_order = data.sort.unwrap_or_default();
    messages.sort_by(|a, b| match sort_order {
        v0::SortOrder::Asc => a.id.cmp(&b.id),
        v0::SortOrder::Desc => b.id.cmp(&a.id),
    });

    // TODO: abstract the user and member fetching logic out
    let response = if let Some(true) = data.include_users {
        let user_ids = messages
            .iter()
            .flat_map(|m| {
                let mut users = vec![m.author.clone()];
                if let Some(system) = &m.system {
                    match system {
                        v0::SystemMessage::ChannelDescriptionChanged { by } => {
                            users.push(by.clone())
                        }
                        v0::SystemMessage::ChannelIconChanged { by } => users.push(by.clone()),
                        v0::SystemMessage::ChannelOwnershipChanged { from, to, .. } => {
                            users.push(from.clone());
                            users.push(to.clone())
                        }
                        v0::SystemMessage::ChannelRenamed { by, .. } => users.push(by.clone()),
                        v0::SystemMessage::UserAdded { by, id, .. }
                        | v0::SystemMessage::UserRemove { by, id, .. } => {
                            users.push(by.clone());
                            users.push(id.clone());
                        }
                        v0::SystemMessage::UserBanned { id, .. }
                        | v0::SystemMessage::UserKicked { id, .. }
                        | v0::SystemMessage::UserJoined { id, .. }
                        | v0::SystemMessage::UserLeft { id, .. } => {
                            users.push(id.clone());
                        }
                        v0::SystemMessage::Text { .. } => {}
                        v0::SystemMessage::MessagePinned { by, .. } => {
                            users.push(by.clone());
                        }
                        v0::SystemMessage::MessageUnpinned { by, .. } => {
                            users.push(by.clone());
                        }
                        v0::SystemMessage::CallStarted { by, .. } => users.push(by.clone()),
                    }
                }
                users
            })
            .collect::<HashSet<String>>()
            .into_iter()
            .collect::<Vec<String>>();
        let users = User::fetch_many_ids_as_mutuals(db, &user, &user_ids).await?;

        v0::BulkMessageResponse::MessagesAndUsers {
            messages,
            users,
            members: if let Some(server_id) = server_id {
                Some(
                    db.fetch_members(&server_id, &user_ids)
                        .await?
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                )
            } else {
                None
            },
        }
    } else {
        v0::BulkMessageResponse::JustMessages(messages)
    };

    Ok(Json(response))
}
