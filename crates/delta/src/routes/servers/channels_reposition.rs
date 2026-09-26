use revolt_database::util::permissions::DatabasePermissionQuery;
use revolt_database::{util::reference::Reference, Channel, Database, User};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

/*
if data.parent.is_some() || data.remove.contains(&v0::FieldsChannel::Parent) {
    permissions.throw_if_lacking_channel_permission(ChannelPermission::MoveChannels)?;
}

if let Some(parent) = &data.parent {
    let server = server.unwrap();
    let Some(category) = server.categories.get(parent) else { return Err(create_error!(UnknownCategory)) };

    let mut query = DatabasePermissionQuery::new(db, &user).category(category).server(&server);
    let permissions = calculate_category_permissions(&mut query)
        .await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::MoveChannels)?;
};
*/

/// # Reposition Channels
///
/// Modifies the channel order in a server.
#[openapi(tag = "Server Information")]
#[post("/<server>/channels/positions", data = "<data>")]
pub async fn reposition_channels(
    db: &State<Database>,
    user: User,
    server: Reference<'_>,
    data: Json<Vec<v0::ChannelReposition>>,
) -> Result<Json<v0::Channel>> {
    let data = data.into_inner();

    let mut server = server.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;

    todo!()
}
