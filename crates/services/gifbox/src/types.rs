use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::tenor::types;

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

/// Indivual gif result.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct MediaResult {
    /// Unique Tenor id.
    pub id: String,
    /// Mapping of each file format and url of the file.
    pub media_formats: HashMap<String, MediaObject>,
    /// Public Tenor web url for the gif.
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

impl From<types::PaginatedMediaResponse> for PaginatedMediaResponse {
    fn from(value: types::PaginatedMediaResponse) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(|result| result.into())
                .collect(),
            next: if value.next.is_empty() {
                None
            } else {
                Some(value.next)
            },
        }
    }
}

impl From<types::MediaResponse> for MediaResult {
    fn from(value: types::MediaResponse) -> Self {
        Self {
            id: value.id,
            media_formats: value
                .media_formats
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            url: value.url,
        }
    }
}

impl From<types::MediaObject> for MediaObject {
    fn from(value: types::MediaObject) -> Self {
        Self {
            url: value.url,
            dimensions: value.dims,
        }
    }
}

impl From<types::CategoryResponse> for CategoryResponse {
    fn from(value: types::CategoryResponse) -> Self {
        Self {
            title: value.searchterm,
            image: value.image,
        }
    }
}
