use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, voice::{sync_voice_permissions, VoiceClient}, Database, User
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission, Override};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Set Role Permission
///
/// Sets permissions for the specified role in this channel.
///
/// Channel must be a `TextChannel`.
#[openapi(tag = "Channel Permissions")]
#[put("/<target>/permissions/<role_id>", data = "<data>", rank = 2)]
pub async fn set_role_permissions(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    target: Reference<'_>,
    role_id: String,
    data: Json<v0::DataSetRolePermissions>,
) -> Result<Json<v0::Channel>> {
    let channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions: revolt_permissions::PermissionValue = calculate_channel_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    if let Some(server) = query.server_ref() {
        if let Some(role) = server.roles.get(&role_id) {
            if role.rank <= query.get_member_rank().unwrap_or(i64::MIN) {
                return Err(create_error!(NotElevated));
            }

            let current_value: Override = role.permissions.into();
            permissions
                .throw_permission_override(current_value, &data.permissions)
                .await?;

            let mut new_channel = channel.clone();

            new_channel
                .set_role_permission(db, &role_id, data.permissions.clone().into())
                .await?;

            sync_voice_permissions(db, voice_client, &new_channel, Some(server), Some(&role_id)).await?;

            Ok(Json(new_channel.into()))
        } else {
            Err(create_error!(NotFound))
        }
    } else {
        Err(create_error!(InvalidOperation))
    }
}
