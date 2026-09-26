use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, Database, User
};
use revolt_models::v0;
use revolt_permissions::{calculate_category_permissions, ChannelPermission, Override};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Set Role Permission
///
/// Sets permissions for the specified role in this channel.
///
/// Channel must be a `TextChannel` or `VoiceChannel`.
#[openapi(tag = "Channel Permissions")]
#[put("/<server>/categories/<category>/permissions/<role_id>", data = "<data>", rank = 2)]
pub async fn set_role_permissions(
    db: &State<Database>,
    user: User,
    server: Reference<'_>,
    category: String,
    role_id: String,
    data: Json<v0::DataSetRolePermissions>,
) -> Result<Json<v0::Category>> {
    let mut server = server.as_server(db).await?;
    let mut category = server.categories.get(&category).ok_or(create_error!(UnknownCategory))?.clone();

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server).category(&category);
    let permissions = calculate_category_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    if let Some(role) = server.roles.get(&role_id) {
        if role.rank <= query.get_member_rank().unwrap_or(i64::MIN) {
            return Err(create_error!(NotElevated));
        }

        let current_value: Override = role.permissions.into();
        permissions
            .throw_permission_override(current_value, &data.permissions)
            .await?;

        category
            .set_role_permission(db, &mut server, role_id, data.permissions.clone().into())
            .await?;

        Ok(Json(category.into()))
    } else {
        Err(create_error!(NotFound))
    }
}
