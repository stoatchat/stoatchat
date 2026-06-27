use revolt_database::{util::reference::Reference, Database};
use revolt_models::v0;
use revolt_result::Result;

use rocket::{serde::json::Json, State};

/// # Fetch Emoji
///
/// Fetch an emoji by its id.
#[openapi(tag = "Emojis")]
#[get("/emoji/<emoji_id>")]
pub async fn fetch_emoji(db: &State<Database>, emoji_id: Reference<'_>) -> Result<Json<v0::Emoji>> {
    emoji_id
        .as_emoji(db)
        .await
        .map(|emoji| emoji.into())
        .map(Json)
}
