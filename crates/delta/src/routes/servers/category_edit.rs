use revolt_config::config;
use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, Category, Channel, Database, PartialCategory, PartialChannel, Role, User
};
use revolt_models::v0::{self, DataEditCategory};
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

    let mut category = server.categories
        .get(&category)
        .ok_or(create_error!(UnknownCategory))?
        .clone();

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server).category(&category);
    let permissions = calculate_server_permissions(&mut query)
        .await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;

    let DataEditCategory {
        title,
        mut channels,
        remove
    } = data;

    if channels.is_some() {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::MoveChannels)?;
    }

    // if any channel we are adding to this category are in another category we need to make sure we have MoveChannels in the category before moving it over.
    for category in server.categories.values() {
        if category.channels.iter().any(|c| channels.as_ref().is_some_and(|cs| cs.contains(c))) {
            let mut category_query = query.clone().category(category);

            calculate_server_permissions(&mut category_query)
                .await
                .throw_if_lacking_channel_permission(ChannelPermission::MoveChannels)?;
        }
    }

    // remove the channels from any existing categories to avoid it having two
    for category in server.categories.values_mut() {
        category.channels.retain(|c| channels.as_ref().is_some_and(|cs| cs.contains(c)));
    }

    // only keep channels which exist in the server
    if let Some(channels) = &mut channels {
        channels.retain(|c| server.channels.contains(c));
    }

    // unset parent from all channels which are removed from the category
    for channel_id in &category.channels {
        if channels.as_ref().is_some_and(|cs| !cs.contains(channel_id)) {
            db.update_channel(channel_id, &PartialChannel { parent: None, ..Default::default() }, Vec::new()).await?;
        };
    };

    // update the category with the new values
    category.update(
        db,
        &mut server,
        PartialCategory {
            title,
            channels: channels.clone(),
            ..Default::default()
        },
        remove
            .map(|v| v.into_iter().map(Into::into).collect())
            .unwrap_or_default()
    ).await?;

    let channels = db.fetch_channels(&channels.unwrap_or_default()).await?;

    // update all channels to have the parent set
    for channel in channels {
        if let Some(parent) = channel.parent() {
            if parent != &category.id {
                db.update_channel(channel.id(), &PartialChannel { parent: Some(category.id.clone()), ..Default::default() }, Vec::new()).await?;
            };
        };
    };

    Ok(Json(category.into()))
}
