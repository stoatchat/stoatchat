use std::collections::HashMap;

use iso8601_timestamp::Timestamp;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;
use revolt_config::config;
use revolt_result::Result;

use crate::{AbstractInbox, ChannelCompositeKey, ChannelUnread, InboxEntry, InboxKind};

#[async_trait]
pub trait AbstractChannelUnreads: AbstractInbox + Sync + Send {
    /// Acknowledge a message, and returns updated channel unread.
    async fn acknowledge_message(
        &self,
        channel_id: &str,
        user_id: &str,
        message_id: &str,
    ) -> Result<Option<ChannelUnread>>;

    /// Acknowledge many channels.
    async fn acknowledge_channels(&self, user_id: &str, channel_ids: &[String]) -> Result<()>;

    /// Add a mention.
    async fn add_mention_to_unread<'a>(
        &self,
        channel_id: &str,
        user_id: &str,
        message_ids: &[String],
    ) -> Result<()>;

    /// Add a mention.
    async fn add_mention_to_many_unreads<'a>(
        &self,
        channel_id: &str,
        user_ids: &[String],
        message_ids: &[String],
    ) -> Result<()>;

    /// Fetch all unreads with mentions for a user.
    async fn fetch_unread_mentions(&self, user_id: &str) -> Result<Vec<ChannelUnread>> {
        let unreads = self.fetch_unreads(user_id).await?;
        Ok(unreads
            .into_iter()
            .filter(|unread| {
                unread
                    .mentions
                    .as_ref()
                    .is_some_and(|mentions| !mentions.is_empty())
            })
            .collect())
    }

    /// Fetch all channel unreads for a user.
    async fn fetch_unreads(&self, user_id: &str) -> Result<Vec<ChannelUnread>> {
        let mut unreads = self.fetch_unreads_raw(user_id).await?;
        let inbox = fetch_mention_entries(self, user_id, None).await?;
        apply_inbox_mentions(user_id, &mut unreads, inbox);
        Ok(unreads)
    }

    /// Fetch unread for a specific user in a channel.
    async fn fetch_unread(&self, user_id: &str, channel_id: &str) -> Result<Option<ChannelUnread>> {
        let mut unreads: Vec<ChannelUnread> = self
            .fetch_unread_raw(user_id, channel_id)
            .await?
            .into_iter()
            .collect();
        let inbox = fetch_mention_entries(self, user_id, Some(channel_id)).await?;
        apply_inbox_mentions(user_id, &mut unreads, inbox);
        Ok(unreads.into_iter().next())
    }

    /// Read the unread row without deriving mention state.
    async fn fetch_unreads_raw(&self, user_id: &str) -> Result<Vec<ChannelUnread>>;

    /// Read one unread row without deriving mention state.
    async fn fetch_unread_raw(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Result<Option<ChannelUnread>>;
}

/// Fetch the direct-mention inbox rows that are still inside the retention window.
async fn fetch_mention_entries<T>(
    db: &T,
    user_id: &str,
    channel_id: Option<&str>,
) -> Result<Vec<InboxEntry>>
where
    T: AbstractInbox + ?Sized,
{
    let days = config().await.features.limits.global.inbox_retention_days;
    let after = if days == 0 {
        Timestamp::UNIX_EPOCH
    } else {
        let now = Timestamp::now_utc()
            .duration_since(Timestamp::UNIX_EPOCH)
            .whole_milliseconds()
            .max(0) as i64;
        let cutoff = (now - days as i64 * 86_400_000).max(0);
        Timestamp::UNIX_EPOCH + iso8601_timestamp::Duration::milliseconds(cutoff)
    };

    db.fetch_inbox_items(user_id, &[InboxKind::Mention], after, channel_id, None, i64::MAX)
        .await
}

/// Merge unread direct mentions from the inbox into each channel's mention list.
fn apply_inbox_mentions(user_id: &str, unreads: &mut Vec<ChannelUnread>, inbox: Vec<InboxEntry>) {
    let mut by_channel: HashMap<String, Vec<String>> = HashMap::new();
    for entry in inbox {
        if entry.entry_kind == InboxKind::Mention {
            by_channel
                .entry(entry.channel_id)
                .or_default()
                .push(entry.message_id);
        }
    }

    for unread in unreads.iter_mut() {
        let last_id = unread.last_id.clone();
        let mut mentions = unread.mentions.take().unwrap_or_default();

        if let Some(ids) = by_channel.remove(&unread.id.channel) {
            mentions.extend(ids);
        }

        if let Some(last_id) = last_id.as_deref() {
            mentions.retain(|id| id.as_str() > last_id);
        }

        mentions.sort_unstable();
        mentions.dedup();
        unread.mentions = (!mentions.is_empty()).then_some(mentions);
    }

    // A channel can have an unread inbox entry before it has a channel_unreads
    // row (for example the first ever mention in that channel).
    for (channel_id, mut mentions) in by_channel {
        mentions.sort_unstable();
        mentions.dedup();

        if mentions.is_empty() {
            continue;
        }

        unreads.push(ChannelUnread {
            id: ChannelCompositeKey {
                channel: channel_id,
                user: user_id.to_string(),
            },
            last_id: None,
            mentions: Some(mentions),
        });
    }
}
