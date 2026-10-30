use revolt_database::{
    util::permissions::DatabasePermissionQuery, Channel, Database, Server, User,
};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission, Override};
use revolt_result::{create_error, Result};

/// Prevent a non-owner from locking themselves out of a channel.
pub async fn throw_if_would_lose_channel_view(
    db: &Database,
    user: &User,
    channel: &Channel,
    role_id: Option<&str>,
    permissions: &Override,
) -> Result<()> {
    if !matches!(channel, Channel::TextChannel { .. }) {
        return Ok(());
    }

    let mut channel = channel.clone();

    match &mut channel {
        Channel::TextChannel {
            role_permissions,
            default_permissions,
            ..
        } => {
            if let Some(role_id) = role_id {
                role_permissions.insert(role_id.to_string(), permissions.clone().into());
            } else {
                *default_permissions = Some(permissions.clone().into());
            }
        }
        _ => unreachable!(),
    }

    let mut query = DatabasePermissionQuery::new(db, user).channel(&channel);
    let permissions = calculate_channel_permissions(&mut query).await;

    if !permissions.has_channel_permission(ChannelPermission::ViewChannel) {
        return Err(create_error!(CannotRemoveOwnChannelView));
    }

    Ok(())
}

/// Prevent a non-owner from locking themselves out of the server's channels.
pub async fn throw_if_would_lose_server_channel_view(
    db: &Database,
    user: &User,
    server: &Server,
    role_id: &str,
    permissions: &Override,
) -> Result<()> {
    if server.owner == user.id {
        return Ok(());
    }

    let mut modified = server.clone();
    match modified.roles.get_mut(role_id) {
        Some(role) => role.permissions = permissions.clone().into(),
        None => return Ok(()),
    }

    let channels = db.fetch_channels(&server.channels).await?;

    for channel in channels {
        if !matches!(channel, Channel::TextChannel { .. }) {
            continue;
        }

        let mut old_query = DatabasePermissionQuery::new(db, user)
            .server(server)
            .channel(&channel);
        if !calculate_channel_permissions(&mut old_query)
            .await
            .has_channel_permission(ChannelPermission::ViewChannel)
        {
            continue;
        }

        let mut new_query = DatabasePermissionQuery::new(db, user)
            .server(&modified)
            .channel(&channel);
        if !calculate_channel_permissions(&mut new_query)
            .await
            .has_channel_permission(ChannelPermission::ViewChannel)
        {
            return Err(create_error!(CannotRemoveOwnChannelView));
        }
    }

    Ok(())
}
