use bson::Document;
use revolt_result::Result;

use crate::{Emoji, PartialEmoji};
use crate::MongoDb;

use super::AbstractEmojis;

static COL: &str = "emojis";

#[async_trait]
impl AbstractEmojis for MongoDb {
    /// Insert emoji into database.
    async fn insert_emoji(&self, emoji: &Emoji) -> Result<()> {
        query!(self, insert_one, COL, &emoji).map(|_| ())
    }

    /// Fetch an emoji by its id
    async fn fetch_emoji(&self, id: &str) -> Result<Emoji> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch emoji by their parent id
    async fn fetch_emoji_by_parent_id(&self, parent_id: &str) -> Result<Vec<Emoji>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "parent.id": parent_id
            }
        )
    }

    /// Fetch emoji by their parent ids
    async fn fetch_emoji_by_parent_ids(&self, parent_ids: &[String]) -> Result<Vec<Emoji>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "parent.id": {
                    "$in": parent_ids
                }
            }
        )
    }

    /// Update emoji with new information
    async fn update_emoji(&self, emoji_id: &str, partial: &PartialEmoji) -> Result<()> {
        query!(self, update_one_by_id, COL, emoji_id, partial, vec![], None).map(|_| ())
    }

    /// Detach an emoji by its id
    async fn detach_emoji(&self, emoji: &Emoji) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": &emoji.id
                },
                doc! {
                    "$set": {
                        "parent": {
                            "type": "Detached"
                        }
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }
}
