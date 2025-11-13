use crate::{
    calculate_channel_permissions, calculate_user_permissions, ChannelPermission, ChannelType,
    Override, PermissionQuery, RelationshipStatus, DEFAULT_PERMISSION_DIRECT_MESSAGE,
    DEFAULT_PERMISSION_SERVER, DEFAULT_PERMISSION_VIEW_ONLY,
};

#[async_std::test]
async fn validate_user_permissions() {
    /// Scenario in which we are friends with a user
    /// and we have a DM channel open with them
    struct Scenario {}
    let mut query = Scenario {};

    let perms = calculate_user_permissions(&mut query).await;
    assert!(perms.has(u64::MAX));

    let perms = calculate_channel_permissions(&mut query).await;
    let value: u64 = perms.into();
    assert_eq!(value, *DEFAULT_PERMISSION_DIRECT_MESSAGE);

    #[async_trait]
    impl PermissionQuery for Scenario {
        async fn are_we_privileged(&mut self) -> bool {
            false
        }

        async fn are_we_a_bot(&mut self) -> bool {
            false
        }

        async fn are_the_users_same(&mut self) -> bool {
            false
        }

        async fn user_relationship(&mut self) -> RelationshipStatus {
            RelationshipStatus::Friend
        }

        async fn user_is_bot(&mut self) -> bool {
            false
        }

        async fn have_mutual_connection(&mut self) -> bool {
            false
        }

        async fn are_we_server_owner(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_a_member(&mut self) -> bool {
            unreachable!()
        }

        async fn get_default_server_permissions(&mut self) -> u64 {
            unreachable!()
        }

        async fn get_our_server_role_overrides(&mut self) -> Vec<Override> {
            unreachable!()
        }

        async fn are_we_timed_out(&mut self) -> bool {
            unreachable!()
        }

        async fn do_we_have_publish_overwrites(&mut self) -> bool {
            true
        }

        async fn do_we_have_receive_overwrites(&mut self) -> bool {
            true
        }

        async fn get_channel_type(&mut self) -> ChannelType {
            ChannelType::DirectMessage
        }

        async fn get_default_channel_permissions(&mut self) -> Override {
            unreachable!()
        }

        async fn get_our_channel_role_overrides(&mut self) -> Vec<Override> {
            unreachable!()
        }

        async fn do_we_own_the_channel(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_part_of_the_channel(&mut self) -> bool {
            true
        }

        async fn set_recipient_as_user(&mut self) {
            // no-op
        }

        async fn set_server_from_channel(&mut self) {
            unreachable!()
        }
    }
}

#[async_std::test]
async fn validate_group_permissions() {
    /// Scenario in which we are in a group channel with only talking permission
    struct Scenario {}
    let mut query = Scenario {};

    let perms = calculate_channel_permissions(&mut query).await;
    let value: u64 = perms.into();
    assert_eq!(
        value,
        *DEFAULT_PERMISSION_VIEW_ONLY | ChannelPermission::SendMessage as u64
    );

    #[async_trait]
    impl PermissionQuery for Scenario {
        async fn are_we_privileged(&mut self) -> bool {
            false
        }

        async fn are_we_a_bot(&mut self) -> bool {
            unreachable!()
        }

        async fn are_the_users_same(&mut self) -> bool {
            unreachable!()
        }

        async fn user_relationship(&mut self) -> RelationshipStatus {
            unreachable!()
        }

        async fn user_is_bot(&mut self) -> bool {
            unreachable!()
        }

        async fn have_mutual_connection(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_server_owner(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_a_member(&mut self) -> bool {
            unreachable!()
        }

        async fn get_default_server_permissions(&mut self) -> u64 {
            unreachable!()
        }

        async fn get_our_server_role_overrides(&mut self) -> Vec<Override> {
            unreachable!()
        }

        async fn are_we_timed_out(&mut self) -> bool {
            unreachable!()
        }

        async fn do_we_have_publish_overwrites(&mut self) -> bool {
            true
        }

        async fn do_we_have_receive_overwrites(&mut self) -> bool {
            true
        }

        async fn get_channel_type(&mut self) -> ChannelType {
            ChannelType::Group
        }

        async fn get_default_channel_permissions(&mut self) -> Override {
            Override {
                allow: ChannelPermission::SendMessage as u64,
                deny: 0,
            }
        }

        async fn get_our_channel_role_overrides(&mut self) -> Vec<Override> {
            unreachable!()
        }

        async fn do_we_own_the_channel(&mut self) -> bool {
            false
        }

        async fn are_we_part_of_the_channel(&mut self) -> bool {
            true
        }

        async fn set_recipient_as_user(&mut self) {
            unreachable!()
        }

        async fn set_server_from_channel(&mut self) {
            unreachable!()
        }
    }
}

#[async_std::test]
async fn validate_server_permissions() {
    /// Scenario in which we are in a server channel where:
    /// - the server grants reading history and sending messages by default
    /// - we have a role that allows us to upload files and react but denies reading history
    /// - however the channel disallows sending messages
    /// - and removes our role specific react permission
    struct Scenario {}
    let mut query = Scenario {};

    let perms = calculate_channel_permissions(&mut query).await;
    let value: u64 = perms.into();
    assert_eq!(
        value,
        ChannelPermission::ViewChannel as u64 | ChannelPermission::UploadFiles as u64
    );

    #[async_trait]
    impl PermissionQuery for Scenario {
        async fn are_we_privileged(&mut self) -> bool {
            false
        }

        async fn are_we_a_bot(&mut self) -> bool {
            unreachable!()
        }

        async fn are_the_users_same(&mut self) -> bool {
            unreachable!()
        }

        async fn user_relationship(&mut self) -> RelationshipStatus {
            unreachable!()
        }

        async fn user_is_bot(&mut self) -> bool {
            unreachable!()
        }

        async fn have_mutual_connection(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_server_owner(&mut self) -> bool {
            false
        }

        async fn are_we_a_member(&mut self) -> bool {
            true
        }

        async fn get_default_server_permissions(&mut self) -> u64 {
            ChannelPermission::ViewChannel as u64
                | ChannelPermission::SendMessage as u64
                | ChannelPermission::ReadMessageHistory as u64
        }

        async fn get_our_server_role_overrides(&mut self) -> Vec<Override> {
            vec![Override {
                allow: ChannelPermission::UploadFiles as u64 | ChannelPermission::React as u64,
                deny: ChannelPermission::ReadMessageHistory as u64,
            }]
        }

        async fn are_we_timed_out(&mut self) -> bool {
            false
        }

        async fn do_we_have_publish_overwrites(&mut self) -> bool {
            true
        }

        async fn do_we_have_receive_overwrites(&mut self) -> bool {
            true
        }

        async fn get_channel_type(&mut self) -> ChannelType {
            ChannelType::ServerChannel
        }

        async fn get_default_channel_permissions(&mut self) -> Override {
            Override {
                allow: 0,
                deny: ChannelPermission::SendMessage as u64,
            }
        }

        async fn get_our_channel_role_overrides(&mut self) -> Vec<Override> {
            vec![Override {
                allow: 0,
                deny: ChannelPermission::React as u64,
            }]
        }

        async fn do_we_own_the_channel(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_part_of_the_channel(&mut self) -> bool {
            unreachable!()
        }

        async fn set_recipient_as_user(&mut self) {
            unreachable!()
        }

        async fn set_server_from_channel(&mut self) {
            // no-op
        }
    }
}

#[async_std::test]
async fn validate_timed_out_member() {
    /// Scenario in which we are in a server that we have been timed out from
    struct Scenario {}
    let mut query = Scenario {};

    let perms = calculate_channel_permissions(&mut query).await;
    let value: u64 = perms.into();
    assert_eq!(value, *DEFAULT_PERMISSION_VIEW_ONLY);

    #[async_trait]
    impl PermissionQuery for Scenario {
        async fn are_we_privileged(&mut self) -> bool {
            false
        }

        async fn are_we_a_bot(&mut self) -> bool {
            unreachable!()
        }

        async fn are_the_users_same(&mut self) -> bool {
            unreachable!()
        }

        async fn user_relationship(&mut self) -> RelationshipStatus {
            unreachable!()
        }

        async fn user_is_bot(&mut self) -> bool {
            unreachable!()
        }

        async fn have_mutual_connection(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_server_owner(&mut self) -> bool {
            false
        }

        async fn are_we_a_member(&mut self) -> bool {
            true
        }

        async fn get_default_server_permissions(&mut self) -> u64 {
            *DEFAULT_PERMISSION_SERVER
        }

        async fn get_our_server_role_overrides(&mut self) -> Vec<Override> {
            vec![]
        }

        async fn are_we_timed_out(&mut self) -> bool {
            true
        }

        async fn do_we_have_publish_overwrites(&mut self) -> bool {
            true
        }

        async fn do_we_have_receive_overwrites(&mut self) -> bool {
            true
        }

        async fn get_channel_type(&mut self) -> ChannelType {
            ChannelType::ServerChannel
        }

        async fn get_default_channel_permissions(&mut self) -> Override {
            Override { allow: 0, deny: 0 }
        }

        async fn get_our_channel_role_overrides(&mut self) -> Vec<Override> {
            vec![]
        }

        async fn do_we_own_the_channel(&mut self) -> bool {
            unreachable!()
        }

        async fn are_we_part_of_the_channel(&mut self) -> bool {
            unreachable!()
        }

        async fn set_recipient_as_user(&mut self) {
            unreachable!()
        }

        async fn set_server_from_channel(&mut self) {
            // no-op
        }
    }
}
