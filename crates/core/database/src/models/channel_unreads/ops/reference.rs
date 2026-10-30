use revolt_result::Result;
use ulid::Ulid;

use crate::{ChannelCompositeKey, ChannelUnread, ReferenceDb};

use super::AbstractChannelUnreads;

#[async_trait]
impl AbstractChannelUnreads for ReferenceDb {
    /// Acknowledge a message.
    async fn acknowledge_message(
        &self,
        channel_id: &str,
        user_id: &str,
        message_id: &str,
    ) -> Result<Option<ChannelUnread>> {
        let mut unreads = self.channel_unreads.lock().await;
        let key = ChannelCompositeKey {
            channel: channel_id.to_string(),
            user: user_id.to_string(),
        };

        if let Some(unread) = unreads.get_mut(&key) {
            unread.mentions = None;
            unread.last_id.replace(message_id.to_string());
        } else {
            unreads.insert(
                key.clone(),
                ChannelUnread {
                    id: key.clone(),
                    last_id: Some(message_id.to_string()),
                    mentions: None,
                },
            );
        }

        Ok(unreads.get(&key).cloned())
    }

    /// Acknowledge many channels.
    async fn acknowledge_channels(&self, user_id: &str, channel_ids: &[String]) -> Result<()> {
        let current_time = Ulid::new().to_string();
        for channel_id in channel_ids {
            #[allow(clippy::disallowed_methods)]
            self.acknowledge_message(channel_id, user_id, &current_time)
                .await?;
        }

        Ok(())
    }

    /// Add a mention.
    async fn add_mention_to_unread<'a>(
        &self,
        channel_id: &str,
        user_id: &str,
        message_ids: &[String],
    ) -> Result<()> {
        let mut unreads = self.channel_unreads.lock().await;
        let key = ChannelCompositeKey {
            channel: channel_id.to_string(),
            user: user_id.to_string(),
        };

        if let Some(unread) = unreads.get_mut(&key) {
            unread.mentions.replace(message_ids.to_vec());
        } else {
            unreads.insert(
                key.clone(),
                ChannelUnread {
                    id: key,
                    last_id: None,
                    mentions: Some(message_ids.to_vec()),
                },
            );
        }

        Ok(())
    }

    /// Add a mention to multiple users.
    async fn add_mention_to_many_unreads<'a>(
        &self,
        channel_id: &str,
        user_ids: &[String],
        message_ids: &[String],
    ) -> Result<()> {
        let mut unreads = self.channel_unreads.lock().await;

        for user_id in user_ids {
            let key = ChannelCompositeKey {
                channel: channel_id.to_string(),
                user: user_id.to_string(),
            };

            if let Some(unread) = unreads.get_mut(&key) {
                unread.mentions.replace(message_ids.to_vec());
            } else {
                unreads.insert(
                    key.clone(),
                    ChannelUnread {
                        id: key,
                        last_id: None,
                        mentions: Some(message_ids.to_vec()),
                    },
                );
            }
        }

        Ok(())
    }

    /// Remove specific message IDs from stored mention lists for the given users.
    async fn remove_mentions_for_users(
        &self,
        channel_id: &str,
        user_ids: &[String],
        message_ids: &[String],
    ) -> Result<()> {
        if user_ids.is_empty() || message_ids.is_empty() {
            return Ok(());
        }

        let mut unreads = self.channel_unreads.lock().await;
        for unread in unreads.values_mut() {
            if unread.id.channel != channel_id || !user_ids.contains(&unread.id.user) {
                continue;
            }

            let empty = match unread.mentions.as_mut() {
                Some(mentions) => {
                    mentions.retain(|id| !message_ids.contains(id));
                    mentions.is_empty()
                }
                None => false,
            };

            if empty {
                unread.mentions = None;
            }
        }

        Ok(())
    }

    /// Remove specific message IDs from stored mention lists across one channel.
    async fn remove_mentions_in_channel(
        &self,
        channel_id: &str,
        message_ids: &[String],
    ) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let mut unreads = self.channel_unreads.lock().await;
        for unread in unreads.values_mut() {
            if unread.id.channel != channel_id {
                continue;
            }

            let empty = match unread.mentions.as_mut() {
                Some(mentions) => {
                    mentions.retain(|id| !message_ids.contains(id));
                    mentions.is_empty()
                }
                None => false,
            };

            if empty {
                unread.mentions = None;
            }
        }

        Ok(())
    }

    /// Remove specific message IDs from stored mention lists across all rows.
    async fn remove_mentions_for_messages(&self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let mut unreads = self.channel_unreads.lock().await;
        for unread in unreads.values_mut() {
            let empty = match unread.mentions.as_mut() {
                Some(mentions) => {
                    mentions.retain(|id| !message_ids.contains(id));
                    mentions.is_empty()
                }
                None => false,
            };

            if empty {
                unread.mentions = None;
            }
        }

        Ok(())
    }

    /// Read all channel unreads for a user without deriving mentions.
    async fn fetch_unreads_raw(&self, user_id: &str) -> Result<Vec<ChannelUnread>> {
        let unreads = self.channel_unreads.lock().await;
        Ok(unreads
            .values()
            .filter(|unread| unread.id.user == user_id)
            .cloned()
            .collect())
    }

    /// Read one channel unread without deriving mentions.
    async fn fetch_unread_raw(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<Option<ChannelUnread>> {
        let unreads = self.channel_unreads.lock().await;

        Ok(unreads
            .get(&ChannelCompositeKey {
                channel: channel_id.to_string(),
                user: user_id.to_string(),
            })
            .cloned())
    }
}
