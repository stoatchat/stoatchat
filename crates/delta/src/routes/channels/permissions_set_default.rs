use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, voice::{sync_voice_permissions, VoiceClient}, Channel, Database, PartialChannel, User
};
use revolt_models::v0::{self, DataDefaultChannelPermissions};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Set Default Permission
///
/// Sets permissions for the default role in this channel.
///
/// Channel must be a `Group` or `TextChannel`.
#[openapi(tag = "Channel Permissions")]
#[put("/<target>/permissions/default", data = "<data>", rank = 1)]
pub async fn set_default_channel_permissions(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::DataDefaultChannelPermissions>,
) -> Result<Json<v0::Channel>> {
    let data = data.into_inner();

    let mut channel = target.as_channel(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;

    permissions.throw_if_lacking_channel_permission(ChannelPermission::ManagePermissions)?;

    match &channel {
        Channel::Group { .. } => {
            if let DataDefaultChannelPermissions::Value { permissions } = data {
                channel
                    .update(
                        db,
                        PartialChannel {
                            permissions: Some(permissions as i64),
                            ..Default::default()
                        },
                        vec![],
                    )
                    .await?;
            } else {
                return Err(create_error!(InvalidOperation));
            }
        }
        Channel::TextChannel {
            default_permissions,
            ..
        } => {
            if let DataDefaultChannelPermissions::Field { permissions: field } = data {
                permissions
                    .throw_permission_override(default_permissions.map(|x| x.into()), &field)
                    .await?;

                channel
                    .update(
                        db,
                        PartialChannel {
                            default_permissions: Some(field.into()),
                            ..Default::default()
                        },
                        vec![],
                    )
                    .await?;
            } else {
                return Err(create_error!(InvalidOperation));
            }
        }
        _ => return Err(create_error!(InvalidOperation)),
    }

    let server = match channel.server() {
        Some(server_id) => Some(Reference::from_unchecked(server_id).as_server(db).await?),
        None => None
    };

    sync_voice_permissions(db, voice_client, &channel, server.as_ref(), None).await?;

    Ok(Json(channel.into()))
}
