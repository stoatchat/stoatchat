use std::collections::hash_map::Entry;

use super::AbstractChannels;
use crate::ReferenceDb;
use crate::{Channel, FieldsChannel, PartialChannel};
use revolt_permissions::OverrideField;
use revolt_result::Result;

#[async_trait]
impl AbstractChannels for ReferenceDb {
    /// Insert a new channel in the database
    async fn insert_channel(&self, channel: &Channel) -> Result<()> {
        let mut channels = self.channels.lock().await;
        if let Entry::Vacant(entry) = channels.entry(channel.id().to_string()) {
            entry.insert(channel.clone());
            Ok(())
        } else {
            Err(create_database_error!("insert", "channel"))
        }
    }

    /// Fetch a channel from the database
    async fn fetch_channel(&self, channel_id: &str) -> Result<Channel> {
        let channels = self.channels.lock().await;
        channels
            .get(channel_id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all channels from the database
    async fn fetch_channels<'a>(&self, ids: &'a [String]) -> Result<Vec<Channel>> {
        let channels = self.channels.lock().await;
        ids.iter()
            .map(|id| {
                channels
                    .get(id)
                    .cloned()
                    .ok_or_else(|| create_error!(NotFound))
            })
            .collect()
    }

    /// Fetch all direct messages for a user
    async fn find_direct_messages(&self, user_id: &str) -> Result<Vec<Channel>> {
        let channels = self.channels.lock().await;
        Ok(channels
            .values()
            .filter(|channel| channel.contains_user(user_id))
            .cloned()
            .collect())
    }

    // Fetch saved messages channel
    async fn find_saved_messages_channel(&self, user_id: &str) -> Result<Channel> {
        let channels = self.channels.lock().await;
        channels
            .get(user_id)
            .cloned()
            .ok_or_else(|| create_database_error!("fetch", "channel"))
    }

    // Fetch direct message channel (DM or Saved Messages)
    async fn find_direct_message_channel(&self, user_a: &str, user_b: &str) -> Result<Channel> {
        let channels = self.channels.lock().await;
        for (_, data) in channels.iter() {
            if data.contains_user(user_a) && data.contains_user(user_b) {
                return Ok(data.to_owned());
            }
        }
        Err(create_error!(NotFound))
    }
    /// Insert a user to a group
    async fn add_user_to_group(&self, channel_id: &str, user_id: &str) -> Result<()> {
        let mut channels = self.channels.lock().await;

        if let Some(Channel::Group { recipients, .. }) = channels.get_mut(channel_id) {
            recipients.push(String::from(user_id));
            Ok(())
        } else {
            Err(create_error!(InvalidOperation))
        }
    }
    /// Insert channel role permissions
    async fn set_channel_role_permission(
        &self,
        channel_id: &str,
        role_id: &str,
        permissions: OverrideField,
    ) -> Result<()> {
        let mut channels = self.channels.lock().await;

        if let Some(mut channel) = channels.get_mut(channel_id) {
            match &mut channel {
                Channel::TextChannel {
                    role_permissions, ..
                } => {
                    if role_permissions.get(role_id).is_some() {
                        role_permissions.remove(role_id);
                        role_permissions.insert(String::from(role_id), permissions);

                        Ok(())
                    } else {
                        Err(create_error!(NotFound))
                    }
                }
                _ => Err(create_error!(NotFound)),
            }
        } else {
            Err(create_error!(NotFound))
        }
    }

    // Update channel
    async fn update_channel(
        &self,
        id: &str,
        channel: &PartialChannel,
        remove: Vec<FieldsChannel>,
    ) -> Result<()> {
        let mut channels = self.channels.lock().await;
        if let Some(channel_data) = channels.get_mut(id) {
            channel_data.apply_options(channel.to_owned());
            channel_data.remove_fields(remove);
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    // Remove a user from a group
    async fn remove_user_from_group(&self, channel: &str, user: &str) -> Result<()> {
        let mut channels = self.channels.lock().await;
        if let Some(channel_data) = channels.get_mut(channel) {
            if channel_data.users()?.contains(&String::from(user)) {
                channel_data.users()?.retain(|x| x != user);
                return Ok(());
            } else {
                return Err(create_error!(NotFound));
            }
        }
        Err(create_error!(NotFound))
    }

    // Delete a channel
    async fn delete_channel(&self, channel: &Channel) -> Result<()> {
        let mut channels = self.channels.lock().await;
        if channels.remove(channel.id()).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
