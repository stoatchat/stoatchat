//! Tenor API models

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaginatedMediaResponse {
    pub results: Vec<MediaResponse>,
    pub next: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaObject {
    pub url: String,
    pub dims: Vec<u64>,
    pub duration: f64,
    pub size: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaResponse {
    pub created: f64,
    #[serde(default)]
    pub hasaudio: bool,
    pub id: String,
    pub media_formats: HashMap<String, MediaObject>,
    pub tags: Vec<String>,
    pub title: String,
    pub content_description: String,
    pub itemurl: String,
    #[serde(default)]
    pub hascaption: bool,
    pub flags: Vec<String>,
    pub bg_color: Option<String>,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CategoriesResponse {
    pub locale: String,
    pub tags: Vec<CategoryResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CategoryResponse {
    pub searchterm: String,
    pub path: String,
    pub image: String,
    pub name: String,
}
