//! Giphy API models

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct SearchResponse {
    pub data: Vec<Gif>,
    pub pagination: Pagination,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Gif {
    pub id: String,
    pub url: String,
    pub title: String,
    pub images: Images,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Images {
    pub original: Rendition,
    pub fixed_width: Rendition,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rendition {
    pub url: Option<String>,
    pub mp4: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Pagination {
    pub total_count: u64,
    pub count: u64,
    pub offset: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CategoriesResponse {
    pub data: Vec<Category>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Category {
    pub name: String,
    pub name_encoded: String,
    pub gif: Gif,
}
