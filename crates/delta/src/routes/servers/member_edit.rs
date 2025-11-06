use std::collections::HashSet;

use revolt_database::{
    events::client::EventV1,
    util::{
        permissions::{perms, DatabasePermissionQuery},
        reference::Reference,
    },
    voice::{
        get_channel_node, get_user_voice_channel_in_server, set_channel_node,
        set_user_moved_from_voice, set_user_moved_to_voice, sync_user_voice_permissions,
        VoiceClient,
    },
    Database, File, PartialMember, User,
};
use revolt_models::v0::{self, FieldsMember};

use revolt_permissions::{
    calculate_channel_permissions, calculate_server_permissions, ChannelPermission,
};
use revolt_result::{create_error, Result};
use rocket::{form::validate::Contains, serde::json::Json, State};
use validator::Validate;

/// # Edit Member
///
/// Edit a member by their id.
#[openapi(tag = "Server Members")]
#[patch("/<server>/members/<member>", data = "<data>")]
pub async fn edit(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    server: Reference<'_>,
    member: Reference<'_>,
    data: Json<v0::DataMemberEdit>,
) -> Result<Json<v0::Member>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    // Fetch server and member
    let mut server = server.as_server(db).await?;
    let target_user = member.as_user(db).await?;
    let mut member = member.as_member(db, &server.id).await?;

    // Fetch our currrent permissions
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    let permissions = calculate_server_permissions(&mut query).await;

    // Fetch target permissions
    let mut target_query = DatabasePermissionQuery::new(db, &target_user).server(&server).member(&member);
    let target_permissions = calculate_server_permissions(&mut target_query).await;

    // Check permissions in server
    if data.nickname.is_some() || data.remove.contains(&v0::FieldsMember::Nickname) {
        if user.id == member.id.user {
            permissions.throw_if_lacking_channel_permission(ChannelPermission::ChangeNickname)?;
        } else {
            permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageNicknames)?;
        }
    }

    if data.avatar.is_some() || data.remove.contains(&v0::FieldsMember::Avatar) {
        if user.id == member.id.user {
            permissions.throw_if_lacking_channel_permission(ChannelPermission::ChangeAvatar)?;
        } else {
            return Err(create_error!(InvalidOperation));
        }
    }

    if data.roles.is_some() || data.remove.contains(&v0::FieldsMember::Roles) {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::AssignRoles)?;
    }

    if data.timeout.is_some() || data.remove.contains(&v0::FieldsMember::Timeout) {
        if data.timeout.is_some() {
            if member.id.user == user.id {
                return Err(create_error!(CannotTimeoutYourself));
            }

            if target_permissions.has_channel_permission(ChannelPermission::TimeoutMembers) {
                return Err(create_error!(IsElevated))
            }
        }

        permissions.throw_if_lacking_channel_permission(ChannelPermission::TimeoutMembers)?;
    }

    if data.can_publish.is_some() {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::MuteMembers)?;
    }

    if data.can_receive.is_some() {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::DeafenMembers)?;
    }

    let new_voice_channel = if let Some(new_channel) = &data.voice_channel {
        if !voice_client.is_enabled() {
            return Err(create_error!(LiveKitUnavailable));
        };

        permissions.throw_if_lacking_channel_permission(ChannelPermission::MoveMembers)?;

        // ensure the channel we are moving them to is in the server and is a voice channel

        let channel = Reference::from_unchecked(new_channel)
            .as_channel(db)
            .await
            .map_err(|_| create_error!(UnknownChannel))?;

        if channel.server().is_none_or(|v| v != member.id.server) {
            Err(create_error!(UnknownChannel))?
        }

        if get_user_voice_channel_in_server(&target_user.id, &server.id)
            .await?
            .is_none()
        {
            Err(create_error!(NotConnected))?
        };

        Some(channel)
    } else {
        None
    };

    // Resolve our ranking
    let our_ranking = query.get_member_rank().unwrap_or(i64::MIN);

    // Check that we have permissions to act against this member
    if member.id.user != user.id
        && member.get_ranking(query.server_ref().as_ref().unwrap()) <= our_ranking
    {
        return Err(create_error!(NotElevated));
    }

    // Check permissions against roles in diff
    if let Some(roles) = &data.roles {
        let current_roles = member.roles.iter().collect::<HashSet<&String>>();

        let new_roles = roles.iter().collect::<HashSet<&String>>();
        let added_roles: Vec<&&String> = new_roles.difference(&current_roles).collect();

        for role_id in added_roles {
            if let Some(role) = server.roles.remove(*role_id) {
                if role.rank <= our_ranking {
                    return Err(create_error!(NotElevated));
                }
            } else {
                return Err(create_error!(InvalidRole));
            }
        }
    }

    // Apply edits to the member object
    let v0::DataMemberEdit {
        nickname,
        avatar,
        roles,
        timeout,
        remove,
        can_publish,
        can_receive,
        voice_channel: _,
    } = data;

    let mut partial = PartialMember {
        nickname,
        roles,
        timeout,
        can_publish,
        can_receive,
        ..Default::default()
    };

    // 1. Remove fields from object
    if remove.contains(&v0::FieldsMember::Avatar) {
        if let Some(avatar) = &member.avatar {
            db.mark_attachment_as_deleted(&avatar.id).await?;
        }
    }

    // 2. Apply new avatar
    if let Some(avatar) = avatar {
        partial.avatar = Some(File::use_user_avatar(db, &avatar, &user.id, &user.id).await?);
    }

    let remove_contains_voice = remove.contains(FieldsMember::CanPublish) || remove.contains(FieldsMember::CanReceive);

    member
        .update(db, partial, remove.into_iter().map(Into::into).collect())
        .await?;

    if let Some(new_voice_channel) = new_voice_channel {
        if let Some(channel) = get_user_voice_channel_in_server(&target_user.id, &server.id).await?
        {
            let old_node = get_channel_node(&channel).await?.unwrap();

            let new_node = match get_channel_node(new_voice_channel.id()).await? {
                Some(node) => node,
                None => {
                    set_channel_node(new_voice_channel.id(), &old_node).await?;
                    old_node.clone()
                }
            };

            set_user_moved_from_voice(&channel, new_voice_channel.id(), &target_user.id).await?;
            set_user_moved_to_voice(new_voice_channel.id(), &channel, &target_user.id).await?;

            let mut query = perms(db, &target_user).channel(&new_voice_channel);
            let permissions = calculate_channel_permissions(&mut query).await;

            voice_client
                .create_room(&new_node, &new_voice_channel)
                .await?;
            let token = voice_client
                .create_token(&new_node, db, &target_user, permissions, &new_voice_channel)
                .await?;

            voice_client
                .remove_user(&old_node, &target_user.id, &channel)
                .await?;

            EventV1::UserMoveVoiceChannel {
                node: new_node,
                from: channel,
                to: new_voice_channel.id().to_string(),
                token,
            }
            .private(target_user.id.clone())
            .await;
        };
    } else if can_publish.is_some() || can_receive.is_some() || remove_contains_voice {
        if let Some(channel) = get_user_voice_channel_in_server(&target_user.id, &server.id).await?
        {
            let node = get_channel_node(&channel).await?.unwrap();
            let channel = Reference::from_unchecked(&channel).as_channel(db).await?;

            sync_user_voice_permissions(
                db,
                voice_client,
                &node,
                &user,
                &channel,
                Some(&server),
                None,
            )
            .await?;
        };
    };

    Ok(Json(member.into()))
}
