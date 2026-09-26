use revolt_database::{
    Database, User,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
};
use revolt_models::v0;
use revolt_permissions::{ChannelPermission, PermissionQuery, calculate_channel_permissions};
use revolt_result::{Result, create_error};
use rocket::{State, serde::json::Json};

/// # Fetch Server
///
/// Fetch a server by its id.
#[openapi(tag = "Server Information")]
#[get("/<target>?<options..>")]
pub async fn fetch(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    options: v0::OptionsFetchServer,
) -> Result<Json<v0::FetchServerResponse>> {
    let server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    if !query.are_we_a_member().await {
        return Err(create_error!(NotFound));
    }

    let all_channels = db.fetch_channels(&server.channels).await?;

    if let Some(true) = options.include_channels {
        let mut visible_channels: Vec<v0::Channel> = vec![];

        for channel in &all_channels {
            let mut channel_query = query.clone().channel(channel);
            if calculate_channel_permissions(&mut channel_query)
                .await
                .has_channel_permission(ChannelPermission::ViewChannel)
            {
                visible_channels.push(channel.clone().into());
            }
        }

        Ok(Json(v0::FetchServerResponse::ServerWithChannels {
            server: server
                .into(
                    db,
                    &all_channels.into_iter().map(Into::into).collect::<Vec<_>>(),
                )
                .await,
            channels: visible_channels,
        }))
    } else {
        Ok(Json(v0::FetchServerResponse::JustServer(
            server
                .into(
                    db,
                    &all_channels.into_iter().map(Into::into).collect::<Vec<_>>(),
                )
                .await,
        )))
    }
}
