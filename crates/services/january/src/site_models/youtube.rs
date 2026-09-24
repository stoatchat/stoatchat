use serde::Deserialize;

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct YoutubeOEmbed {
    pub title: String,
    pub author_name: String,
    pub author_url: String,
    // #[serde(rename = "type")]
    // pub kind: String,
    // pub height: u32,
    // pub width: u32,
    // pub version: String,
    pub provider_name: String,
    // pub provider_url: String,
    // pub thumbnail_height: u32,
    // pub thumbnail_width: u32,
    pub thumbnail_url: String,
    // pub html: String,
}
