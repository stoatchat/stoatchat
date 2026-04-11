use std::collections::HashSet;
use std::str::FromStr;

use once_cell::sync::Lazy;
use revolt_models::v0;
use revolt_result::Result;
use ulid::Ulid;

use crate::events::client::EventV1;
use crate::Database;

static PERMISSIBLE_EMOJIS: Lazy<HashSet<String>> = Lazy::new(|| {
    include_str!("unicode_emoji.txt")
        .split('\n')
        .map(|x| x.into())
        .collect()
});

auto_derived!(
    /// Emoji
    pub struct Emoji {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// What owns this emoji
        pub parent: EmojiParent,
        /// Uploader user id
        pub creator_id: String,
        /// Emoji name
        pub name: String,
        /// Whether the emoji is animated
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub animated: bool,
        /// Whether the emoji is marked as nsfw
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub nsfw: bool,
    }

    /// Parent Id of the emoji
    #[serde(tag = "type")]
    pub enum EmojiParent {
        Server { id: String },
        Detached,
    }

    /// Partial representation of an emoji
    pub struct PartialEmoji {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
    }
);

#[allow(clippy::disallowed_methods)]
impl Emoji {
    /// Get parent id
    fn parent(&self) -> &str {
        match &self.parent {
            EmojiParent::Server { id } => id,
            EmojiParent::Detached => "",
        }
    }

    /// Create an emoji
    pub async fn create(&self, db: &Database) -> Result<()> {
        db.insert_emoji(self).await?;

        EventV1::EmojiCreate(self.clone().into())
            .p(self.parent().to_string())
            .await;

        Ok(())
    }

    /// Delete an emoji
    pub async fn delete(self, db: &Database) -> Result<()> {
        EventV1::EmojiDelete {
            id: self.id.to_string(),
        }
        .p(self.parent().to_string())
        .await;

        db.detach_emoji(&self).await
    }

    /// Update an emoji
    pub async fn update(&mut self, db: &Database, partial: PartialEmoji) -> Result<()> {
        if let Some(name) = partial.name.clone() {
            self.name = name;
        }

        db.update_emoji(&self.id, &partial).await?;

        EventV1::EmojiUpdate {
            id: self.id.clone(),
            data: v0::PartialEmoji {
                name: partial.name.clone(),
            },
        }
        .p(self.parent().to_string())
        .await;

        Ok(())
    }

    /// Check whether we can use a given emoji
    pub async fn can_use(db: &Database, emoji: &str) -> Result<bool> {
        if Ulid::from_str(emoji).is_ok() {
            db.fetch_emoji(emoji).await?;
            Ok(true)
        } else {
            Ok(PERMISSIBLE_EMOJIS.contains(emoji))
        }
    }
}
