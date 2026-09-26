use revolt_config::config;
use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, Category, Database, Role, User
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Create a category
///
/// Creates a new server category.
#[openapi(tag = "Server Categories")]
#[post("/<target>/categories", data = "<data>")]
pub async fn create(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::DataCreateCategory>,
) -> Result<Json<v0::Category>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;

    let config = config().await;
    if server.categories.len() >= config.features.limits.global.server_categories {
        return Err(create_error!(TooManyCategories {
            max: config.features.limits.global.server_categories,
        }));
    };

    let category = Category::create(db, &mut server, data).await?;

    Ok(Json(category.into(Vec::new())))
}
