use crate::{ChannelType, Override, RelationshipStatus};

#[async_trait]
pub trait PermissionQuery {
    // * For calculating user permission

    /// Is our perspective user privileged?
    async fn are_we_privileged(&mut self) -> bool;

    /// Is our perspective user a bot?
    async fn are_we_a_bot(&mut self) -> bool;

    /// Is our perspective user and the currently selected user the same?
    async fn are_the_users_same(&mut self) -> bool;

    /// Get the relationship with have with the currently selected user
    async fn user_relationship(&mut self) -> RelationshipStatus;

    /// Whether the currently selected user is a bot
    async fn user_is_bot(&mut self) -> bool;

    /// Do we have a mutual connection with the currently selected user?
    async fn have_mutual_connection(&mut self) -> bool;

    // * For calculating server permission

    /// Is our perspective user the server's owner?
    async fn are_we_server_owner(&mut self) -> bool;

    /// Is our perspective user a member of the server?
    async fn are_we_a_member(&mut self) -> bool;

    /// Get default server permission
    async fn get_default_server_permissions(&mut self) -> u64;

    /// Get the ordered role overrides (from lowest to highest) for this member in this server
    async fn get_our_server_role_overrides(&mut self) -> Vec<Override>;

    /// Is our perspective user timed out on this server?
    async fn are_we_timed_out(&mut self) -> bool;

    /// Is the member muted?
    async fn do_we_have_publish_overwrites(&mut self) -> bool;

    /// Is the member deafend?
    async fn do_we_have_receive_overwrites(&mut self) -> bool;

    // * For calculating channel permission

    /// Get the type of the channel
    async fn get_channel_type(&mut self) -> ChannelType;

    /// Get the default channel permissions
    /// Group channel defaults should be mapped to an allow-only override
    async fn get_default_channel_permissions(&mut self) -> Override;

    /// Get the ordered role overrides (from lowest to highest) for this member in this channel
    async fn get_our_channel_role_overrides(&mut self) -> Vec<Override>;

    /// Do we own this group or saved messages channel if it is one of those?
    async fn do_we_own_the_channel(&mut self) -> bool;

    /// Are we a recipient of this channel?
    async fn are_we_part_of_the_channel(&mut self) -> bool;

    /// Set the current user as the recipient of this channel
    /// (this will only ever be called for DirectMessage channels, use unimplemented!() for other code paths)
    async fn set_recipient_as_user(&mut self);

    /// Set the current server as the server owning this channel
    /// (this will only ever be called for server channels, use unimplemented!() for other code paths)
    async fn set_server_from_channel(&mut self);
}
