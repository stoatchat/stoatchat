use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::giphy::types;

/// Successful root response
#[derive(Serialize, Debug, ToSchema)]
pub struct RootResponse<'a> {
    pub message: &'a str,
    pub version: &'a str,
}

/// Response containing the current results and the id of the next result for pagination.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct PaginatedMediaResponse {
    /// Current gif results.
    pub results: Vec<MediaResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Id of the next result.
    pub next: Option<String>,
}

/// Individual gif result.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct MediaResult {
    /// Unique GIF id.
    pub id: String,
    /// Mapping of each file format and url of the file.
    pub media_formats: HashMap<String, MediaObject>,
    /// Public web url for the gif.
    pub url: String,
}

/// Represents the gif in a certain file format.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct MediaObject {
    /// File url of the gif in a certain format.
    pub url: String,
    /// Width and height of the file in px.
    pub dimensions: Vec<u64>,
}

/// Represents a GIF category
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct CategoryResponse {
    /// Category title
    pub title: String,
    /// Category image
    pub image: String,
}

impl From<types::SearchResponse> for PaginatedMediaResponse {
    fn from(value: types::SearchResponse) -> Self {
        let next_offset = value.pagination.offset + value.pagination.count;
        let has_more = next_offset < value.pagination.total_count;
        Self {
            results: value
                .data
                .into_iter()
                .map(|gif| gif.into())
                .collect(),
            next: if has_more {
                Some(next_offset.to_string())
            } else {
                None
            },
        }
    }
}

impl From<types::Gif> for MediaResult {
    fn from(value: types::Gif) -> Self {
        let mut media_formats = HashMap::new();

        // Map original mp4 → "webm" key (matches existing frontend expectations)
        if let Some(mp4) = &value.images.original.mp4 {
            let width = value.images.original.width.as_deref().and_then(|w| w.parse().ok()).unwrap_or(0);
            let height = value.images.original.height.as_deref().and_then(|h| h.parse().ok()).unwrap_or(0);
            media_formats.insert("webm".to_string(), MediaObject {
                url: mp4.clone(),
                dimensions: vec![width, height],
            });
        }

        // Map fixed_width mp4 → "tinywebm" key (matches existing frontend expectations)
        if let Some(mp4) = &value.images.fixed_width.mp4 {
            let width = value.images.fixed_width.width.as_deref().and_then(|w| w.parse().ok()).unwrap_or(0);
            let height = value.images.fixed_width.height.as_deref().and_then(|h| h.parse().ok()).unwrap_or(0);
            media_formats.insert("tinywebm".to_string(), MediaObject {
                url: mp4.clone(),
                dimensions: vec![width, height],
            });
        }

        Self {
            id: value.id,
            media_formats,
            url: value.url,
        }
    }
}

impl From<types::Category> for CategoryResponse {
    fn from(value: types::Category) -> Self {
        Self {
            title: value.name,
            image: value.gif.images.fixed_width.url.unwrap_or_default(),
        }
    }
}
