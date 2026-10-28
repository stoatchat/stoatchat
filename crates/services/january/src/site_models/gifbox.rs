use serde::Deserialize;

#[derive(Deserialize)]
pub struct GifboxOEmbed {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: Option<String>,
    pub width: usize,
    pub height: usize,
    pub html: String,
    pub thumbnail_url: Option<String>,
}