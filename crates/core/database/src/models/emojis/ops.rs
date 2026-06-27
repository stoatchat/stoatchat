use revolt_result::Result;

use crate::{Emoji, PartialEmoji};

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractEmojis: Sync + Send {
    /// Insert emoji into database.
    async fn insert_emoji(&self, emoji: &Emoji) -> Result<()>;

    /// Fetch an emoji by its id
    async fn fetch_emoji(&self, id: &str) -> Result<Emoji>;

    /// Fetch emoji by their parent id
    async fn fetch_emoji_by_parent_id(&self, parent_id: &str) -> Result<Vec<Emoji>>;

    /// Fetch emoji by their parent ids
    async fn fetch_emoji_by_parent_ids(&self, parent_ids: &[String]) -> Result<Vec<Emoji>>;

    /// Update emoji with new information
    async fn update_emoji(&self, emoji_id: &str, partial: &PartialEmoji) -> Result<()>;

    /// Detach an emoji by its id
    async fn detach_emoji(&self, emoji: &Emoji) -> Result<()>;
}
