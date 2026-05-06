use crate::{revolt_result::Result, Channel, FieldsChannel, PartialChannel};
use revolt_permissions::OverrideField;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractChannels: Sync + Send {
    /// Insert a new channel in the database
    async fn insert_channel(&self, channel: &Channel) -> Result<()>;

    /// Fetch a channel from the database
    async fn fetch_channel(&self, channel_id: &str) -> Result<Channel>;

    /// Fetch all channels from the database
    async fn fetch_channels<'a>(&self, ids: &'a [String]) -> Result<Vec<Channel>>;

    /// Fetch all direct messages for a user
    async fn find_direct_messages(&self, user_id: &str) -> Result<Vec<Channel>>;

    // Fetch all group dms for a user
    async fn find_group_message_channels(&self, user_id: &str) -> Result<Vec<Channel>>;

    // Fetch saved messages channel
    async fn find_saved_messages_channel(&self, user_id: &str) -> Result<Channel>;

    // Fetch direct message channel (DM or Saved Messages)
    async fn find_direct_message_channel(&self, user_a: &str, user_b: &str) -> Result<Channel>;

    /// Insert a user to a group
    async fn add_user_to_group(&self, channel_id: &str, user_id: &str) -> Result<()>;

    /// Insert channel role permissions
    async fn set_channel_role_permission(
        &self,
        channel_id: &str,
        role_id: &str,
        permissions: OverrideField,
    ) -> Result<()>;

    // Update channel
    async fn update_channel(
        &self,
        id: &str,
        channel_id: &PartialChannel,
        remove: Vec<FieldsChannel>,
    ) -> Result<()>;

    // Remove a user from a group
    async fn remove_user_from_group(&self, channel_id: &str, user_id: &str) -> Result<()>;

    // Remove a user from all specified groups
    async fn remove_user_from_groups(&self, channel_ids: Vec<String>, user_id: &str) -> Result<()>;

    // Delete a channel
    async fn delete_channel(&self, channel_id: &Channel) -> Result<()>;
}
