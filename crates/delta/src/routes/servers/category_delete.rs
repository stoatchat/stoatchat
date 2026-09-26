use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, Database, User
};
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Edits a category
///
/// Edits a server category.
#[openapi(tag = "Server Categories")]
#[delete("/<server>/categories/<category>")]
pub async fn delete(
    db: &State<Database>,
    user: User,
    server: Reference<'_>,
    category: String
) -> Result<EmptyResponse> {
    let mut server = server.as_server(db).await?;

    let category = server.categories
        .get(&category)
        .ok_or(create_error!(UnknownCategory))?
        .clone();

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;

    category.delete(db, &mut server).await?;

    Ok(EmptyResponse)
}
