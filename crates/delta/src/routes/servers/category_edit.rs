use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, PartialCategory, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Edits a category
///
/// Edits a server category.
#[openapi(tag = "Server Categories")]
#[patch("/<server>/categories/<category>", data = "<data>")]
pub async fn edit(
    db: &State<Database>,
    user: User,
    server: Reference<'_>,
    category: String,
    data: Json<v0::DataEditCategory>,
) -> Result<Json<v0::Category>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut server = server.as_server(db).await?;

    let mut category = server
        .categories
        .get(&category)
        .ok_or(create_error!(UnknownCategory))?
        .clone();

    let mut query = DatabasePermissionQuery::new(db, &user)
        .server(&server)
        .category(&category);
    let permissions = calculate_server_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;

    let v0::DataEditCategory { title, remove } = data;

    // update the category with the new values
    category
        .update(
            db,
            &mut server,
            PartialCategory {
                title,
                ..Default::default()
            },
            remove
                .map(|v| v.into_iter().map(Into::into).collect())
                .unwrap_or_default(),
        )
        .await?;

    let channels = db
        .find_category_channels(&category.id)
        .await?
        .into_iter()
        .map(|c| c.id().to_string())
        .collect::<Vec<_>>();

    Ok(Json(category.into(channels)))
}
