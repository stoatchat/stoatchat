use rocket::serde::json::Json;
use crate::routes::admin::util::{
    create_audit_action, flatten_authorized_user, user_has_permission,
};
use revolt_database::{util::reference::Reference, AdminAuthorization, Channel, Database, File, PartialChannel, SystemMessage, User, AMQP};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::State;
use validator::Validate;
use revolt_database::voice::{delete_voice_channel, UserVoiceChannel, VoiceClient};

#[openapi(tag = "Admin")]
#[patch("/admin/channels/<channel_id>?<case>", data= "<data>")]
pub async fn admin_channel_edit(
    db: &State<Database>,
    auth: AdminAuthorization,
    voice_client: &State<VoiceClient>,
    amqp: &State<AMQP>,
    channel_id: Reference<'_>,
    user: User,
    data: Json<v0::DataEditChannel>,
    case: Option<&str>
) -> Result<Json<v0::Channel>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let admin = flatten_authorized_user(&auth);
    if !user_has_permission(admin, v0::AdminUserPermissionFlags::ManageChannels) {
        return Err(create_error!(MissingPermission {
            permission: "ManageChannels".to_string()
        }));
    }

    let mut channel = channel_id.as_channel(db).await?;

    if data.name.is_none()
        && data.description.is_none()
        && data.icon.is_none()
        && data.nsfw.is_none()
        && data.owner.is_none()
        && data.voice.is_none()
        && data.slowmode.is_none()
        && data.remove.is_empty()
    {
        return Ok(Json(channel.into()));
    }

    let mut partial: PartialChannel = Default::default();

    // Transfer group ownership
    if let Some(new_owner) = data.owner {
        if let Channel::Group {
            owner, recipients, ..
        } = &mut channel
        {

            // Ensure user is part of group
            if !recipients.contains(&new_owner) {
                return Err(create_error!(NotInGroup));
            }

            // Transfer ownership
            partial.owner = Some(new_owner.to_string());
            let old_owner = std::mem::replace(owner, new_owner.to_string());

            // Notify clients
            SystemMessage::ChannelOwnershipChanged {
                from: old_owner,
                to: new_owner,
            }
        } else {
            return Err(create_error!(InvalidOperation));
        }
            .into_message(channel.id().to_string())
            .send(
                db,
                Some(amqp),
                user.as_author_for_system(),
                None,
                None,
                &channel,
                false,
            )
            .await
            .ok();
    }

    match &mut channel {
        Channel::Group {
            id,
            name,
            description,
            icon,
            nsfw,
            ..
        } => {
            if data.remove.contains(&v0::FieldsChannel::Icon) {
                if let Some(icon) = &icon {
                    db.mark_attachment_as_deleted(&icon.id).await?;
                }
            }

            for field in &data.remove {
                match field {
                    v0::FieldsChannel::Description => {
                        description.take();
                    }
                    v0::FieldsChannel::Icon => {
                        icon.take();
                    }
                    _ => {}
                }
            }

            if let Some(icon_id) = data.icon {
                partial.icon = Some(File::use_channel_icon(db, &icon_id, id, &user.id).await?);
                *icon = partial.icon.clone();
            }

            if let Some(new_name) = data.name {
                *name = new_name.clone();
                partial.name = Some(new_name);
            }

            if let Some(new_description) = data.description {
                partial.description = Some(new_description);
                *description = partial.description.clone();
            }

            if let Some(new_nsfw) = data.nsfw {
                *nsfw = new_nsfw;
                partial.nsfw = Some(new_nsfw);
            }

            // Send out mutation system messages.
            if let Some(name) = &partial.name {
                SystemMessage::ChannelRenamed {
                    name: name.to_string(),
                    by: user.id.clone(),
                }
                    .into_message(channel.id().to_string())
                    .send(
                        db,
                        Some(amqp),
                        user.as_author_for_system(),
                        None,
                        None,
                        &channel,
                        false,
                    )
                    .await
                    .ok();
            }

            if partial.description.is_some() {
                SystemMessage::ChannelDescriptionChanged {
                    by: user.id.clone(),
                }
                    .into_message(channel.id().to_string())
                    .send(
                        db,
                        Some(amqp),
                        user.as_author_for_system(),
                        None,
                        None,
                        &channel,
                        false,
                    )
                    .await
                    .ok();
            }

            if partial.icon.is_some() {
                SystemMessage::ChannelIconChanged {
                    by: user.id.clone(),
                }
                    .into_message(channel.id().to_string())
                    .send(
                        db,
                        Some(amqp),
                        user.as_author_for_system(),
                        None,
                        None,
                        &channel,
                        false,
                    )
                    .await
                    .ok();
            }
        }
        Channel::TextChannel {
            id,
            name,
            description,
            icon,
            nsfw,
            voice,
            slowmode,
            ..
        } => {
            if data.remove.contains(&v0::FieldsChannel::Icon) {
                if let Some(icon) = &icon {
                    db.mark_attachment_as_deleted(&icon.id).await?;
                }
            }

            for field in &data.remove {
                match field {
                    v0::FieldsChannel::Description => {
                        description.take();
                    }
                    v0::FieldsChannel::Icon => {
                        icon.take();
                    }
                    v0::FieldsChannel::Voice => {
                        voice.take();
                    }
                    _ => {}
                }
            }

            if let Some(icon_id) = data.icon {
                partial.icon = Some(File::use_channel_icon(db, &icon_id, id, &user.id).await?);
                *icon = partial.icon.clone();
            }

            if let Some(new_name) = data.name {
                *name = new_name.clone();
                partial.name = Some(new_name);
            }

            if let Some(new_description) = data.description {
                partial.description = Some(new_description);
                *description = partial.description.clone();
            }

            if let Some(new_nsfw) = data.nsfw {
                *nsfw = new_nsfw;
                partial.nsfw = Some(new_nsfw);
            }

            if let Some(new_voice) = data.voice {
                *voice = Some(new_voice.clone().into());
                partial.voice = Some(new_voice.into());
            }

            if let Some(new_slowmode) = data.slowmode {
                *slowmode = Some(new_slowmode);
                partial.slowmode = Some(new_slowmode);
            }
        }
        _ => return Err(create_error!(InvalidOperation)),
    };

    channel
        .update(
            db,
            partial,
            data.remove.into_iter().map(|f| f.into()).collect(),
        )
        .await?;

    if channel.voice().is_none() {
        delete_voice_channel(voice_client, &UserVoiceChannel::from_channel(&channel)).await?;
    }

    create_audit_action(
        &db,
        &user.id,
        v0::AdminAuditItemActions::EditChannel,
        case,
        Some(channel_id.id),
        None,
    )
        .await?;

    Ok(Json(channel.into()))
}