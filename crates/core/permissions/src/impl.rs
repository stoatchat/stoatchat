use crate::{
    ChannelPermission, ChannelType, PermissionQuery, PermissionValue, RelationshipStatus,
    UserPermission, ALLOW_IN_TIMEOUT, DEFAULT_PERMISSION_DIRECT_MESSAGE,
    DEFAULT_PERMISSION_SAVED_MESSAGES, DEFAULT_PERMISSION_VIEW_ONLY,
};

/// Calculate permissions against a user
pub async fn calculate_user_permissions<P: PermissionQuery>(query: &mut P) -> PermissionValue {
    if query.are_we_privileged().await {
        return u64::MAX.into();
    }

    if query.are_the_users_same().await {
        return u64::MAX.into();
    }

    let mut permissions = 0_u64;
    match query.user_relationship().await {
        RelationshipStatus::Friend => return u64::MAX.into(),
        RelationshipStatus::Blocked | RelationshipStatus::BlockedOther => {
            return (UserPermission::Access as u64).into()
        }
        RelationshipStatus::Incoming | RelationshipStatus::Outgoing => {
            permissions = UserPermission::Access as u64;
        }
        _ => {}
    }

    if query.have_mutual_connection().await {
        permissions = UserPermission::Access as u64 + UserPermission::ViewProfile as u64;

        if query.user_is_bot().await || query.are_we_a_bot().await {
            permissions += UserPermission::SendMessage as u64;
        }

        permissions.into()
    } else {
        permissions.into()
    }

    // TODO: add boolean switch for permission for users to globally message a user
    // maybe an enum?
    // PrivacyLevel { Private, Friends, Mutual, Public, Global }

    // TODO: add boolean switch for permission for users to mutually DM a user
}

/// Calculate permissions against a server
pub async fn calculate_server_permissions<P: PermissionQuery>(query: &mut P) -> PermissionValue {
    if query.are_we_privileged().await || query.are_we_server_owner().await {
        return ChannelPermission::GrantAllSafe.into();
    }

    if !query.are_we_a_member().await {
        return 0_u64.into();
    }

    let mut permissions: PermissionValue = query.get_default_server_permissions().await.into();

    for role_override in query.get_our_server_role_overrides().await {
        permissions.apply(role_override);
    }

    if !query.do_we_have_publish_overwrites().await {
        permissions.revoke(ChannelPermission::Speak as u64);
        permissions.revoke(ChannelPermission::Video as u64);
    }

    if !query.do_we_have_receive_overwrites().await {
        permissions.revoke(ChannelPermission::Listen as u64);
    }

    if query.are_we_timed_out().await {
        permissions.restrict(*ALLOW_IN_TIMEOUT);
    }

    permissions
}

/// Calculate permissions against a channel
pub async fn calculate_channel_permissions<P: PermissionQuery>(query: &mut P) -> PermissionValue {
    if query.are_we_privileged().await {
        return ChannelPermission::GrantAllSafe.into();
    }

    match query.get_channel_type().await {
        ChannelType::SavedMessages => {
            if query.do_we_own_the_channel().await {
                DEFAULT_PERMISSION_SAVED_MESSAGES.into()
            } else {
                0_u64.into()
            }
        }
        ChannelType::DirectMessage => {
            if query.are_we_part_of_the_channel().await {
                query.set_recipient_as_user().await;

                let permissions = calculate_user_permissions(query).await;
                if permissions.has_user_permission(UserPermission::SendMessage) {
                    (*DEFAULT_PERMISSION_DIRECT_MESSAGE).into()
                } else {
                    (*DEFAULT_PERMISSION_VIEW_ONLY).into()
                }
            } else {
                0_u64.into()
            }
        }
        ChannelType::Group => {
            if query.do_we_own_the_channel().await {
                ChannelPermission::GrantAllSafe.into()
            } else if query.are_we_part_of_the_channel().await {
                (*DEFAULT_PERMISSION_VIEW_ONLY
                    | query.get_default_channel_permissions().await.allow)
                    .into()
            } else {
                0_u64.into()
            }
        }
        ChannelType::ServerChannel => {
            query.set_server_from_channel().await;

            if query.are_we_server_owner().await {
                ChannelPermission::GrantAllSafe.into()
            } else if query.are_we_a_member().await {
                let mut permissions = calculate_server_permissions(query).await;
                permissions.apply(query.get_default_channel_permissions().await);

                for role_override in query.get_our_channel_role_overrides().await {
                    permissions.apply(role_override);
                }

                if query.are_we_timed_out().await {
                    permissions.restrict(*ALLOW_IN_TIMEOUT);
                }

                if !permissions.has_channel_permission(ChannelPermission::ViewChannel) {
                    permissions.revoke_all();
                }

                permissions
            } else {
                0_u64.into()
            }
        }
        ChannelType::Unknown => 0_u64.into(),
    }
}
