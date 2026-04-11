use revolt_result::Result;

use crate::{Emoji, PartialEmoji};
use crate::EmojiParent;
use crate::ReferenceDb;

use super::AbstractEmojis;

#[async_trait]
impl AbstractEmojis for ReferenceDb {
    /// Insert emoji into database.
    async fn insert_emoji(&self, emoji: &Emoji) -> Result<()> {
        let mut emojis = self.emojis.lock().await;
        if emojis.contains_key(&emoji.id) {
            Err(create_database_error!("insert", "emoji"))
        } else {
            emojis.insert(emoji.id.to_string(), emoji.clone());
            Ok(())
        }
    }

    /// Fetch an emoji by its id
    async fn fetch_emoji(&self, id: &str) -> Result<Emoji> {
        let emojis = self.emojis.lock().await;
        emojis
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch emoji by their parent id
    async fn fetch_emoji_by_parent_id(&self, parent_id: &str) -> Result<Vec<Emoji>> {
        let emojis = self.emojis.lock().await;
        Ok(emojis
            .values()
            .filter(|emoji| match &emoji.parent {
                EmojiParent::Server { id } => id == parent_id,
                _ => false,
            })
            .cloned()
            .collect())
    }

    /// Fetch emoji by their parent ids
    async fn fetch_emoji_by_parent_ids(&self, parent_ids: &[String]) -> Result<Vec<Emoji>> {
        let emojis = self.emojis.lock().await;
        Ok(emojis
            .values()
            .filter(|emoji| match &emoji.parent {
                EmojiParent::Server { id } => parent_ids.contains(id),
                _ => false,
            })
            .cloned()
            .collect())
    }

    /// Update emoji with new information
    async fn update_emoji(&self, emoji_id: &str, partial: &PartialEmoji) -> Result<()> {
        let mut emojis = self.emojis.lock().await;
        if let Some(emoji) = emojis.get_mut(emoji_id) {
            if let Some(name) = partial.name.clone() {
                emoji.name = name;
            }
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Detach an emoji by its id
    async fn detach_emoji(&self, emoji: &Emoji) -> Result<()> {
        let mut emojis = self.emojis.lock().await;
        if let Some(bot) = emojis.get_mut(&emoji.id) {
            bot.parent = EmojiParent::Detached;
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
