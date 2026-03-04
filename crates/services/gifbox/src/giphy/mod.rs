//! Internal Giphy API wrapper

use std::{sync::Arc, time::Duration};

use lru_time_cache::LruCache;
use reqwest::Client;
use revolt_coalesced::{CoalescionService, CoalescionServiceConfig};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

pub mod types;

const GIPHY_API_BASE_URL: &str = "https://api.giphy.com/v1/gifs";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GiphyError {
    HttpError,
}

#[derive(Clone)]
pub struct Giphy {
    pub key: Arc<str>,
    pub client: Client,
    pub coalescion: CoalescionService<String>,
    pub cache: Arc<RwLock<LruCache<String, Arc<types::SearchResponse>>>>,

    pub categories: Arc<RwLock<LruCache<String, Arc<types::CategoriesResponse>>>>,
    pub featured: Arc<RwLock<LruCache<String, Arc<types::SearchResponse>>>>,
}

impl Giphy {
    pub fn new(key: &str) -> Self {
        Self {
            key: Arc::from(key),
            client: Client::new(),
            coalescion: CoalescionService::from_config(CoalescionServiceConfig {
                max_concurrent: Some(100),
                queue_requests: true,
                max_queue: None,
            }),

            // 1 hour, 1k requests
            cache: Arc::new(RwLock::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60 * 60),
                1000,
            ))),

            // 1 day, 1k requests
            categories: Arc::new(RwLock::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60 * 60 * 24),
                1000,
            ))),

            // 1 day, 1k requests
            featured: Arc::new(RwLock::new(LruCache::with_expiry_duration_and_capacity(
                Duration::from_secs(60 * 60 * 24),
                1000,
            ))),
        }
    }

    pub async fn request<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<Arc<T>, GiphyError> {
        let response = self
            .client
            .get(format!("{GIPHY_API_BASE_URL}{path}"))
            .query(query)
            .send()
            .await
            .inspect_err(|e| {
                revolt_config::capture_error(e);
            })
            .map_err(|_| GiphyError::HttpError)?;

        let text = response.text().await.map_err(|e| {
            revolt_config::capture_error(&e);
            GiphyError::HttpError
        })?;

        Ok(Arc::new(serde_json::from_str(&text).unwrap()))
    }

    /// Strip country code from locale (e.g. "en_US" -> "en")
    fn strip_locale(locale: &str) -> &str {
        locale.split('_').next().unwrap_or(locale)
    }

    pub async fn search(
        &self,
        query: &str,
        locale: &str,
        limit: u32,
        offset: u64,
    ) -> Result<Arc<types::SearchResponse>, GiphyError> {
        let unique_key = format!("s:{query}:{locale}:{offset}");

        if self.cache.read().await.contains_key(&unique_key) {
            if let Some(response) = self.cache.write().await.get(&unique_key) {
                return Ok(response.clone());
            }
        }

        let limit_str = limit.to_string();
        let offset_str = offset.to_string();
        let lang = Self::strip_locale(locale);

        let res = self.coalescion.execute(unique_key.clone(), || async move {
            self.request::<types::SearchResponse>(
                "/search",
                &[
                    ("api_key", &self.key),
                    ("q", query),
                    ("limit", &limit_str),
                    ("offset", &offset_str),
                    ("rating", "g"),
                    ("lang", lang),
                ]
            ).await
        })
        .await
        .unwrap();

        if let Ok(resp) = &*res {
            self.cache.write().await.insert(unique_key, resp.clone());
        }

        (*res).clone()
    }

    pub async fn categories(
        &self,
    ) -> Result<Arc<types::CategoriesResponse>, GiphyError> {
        let unique_key = "categories".to_string();

        if self.categories.read().await.contains_key(&unique_key) {
            if let Some(response) = self.categories.write().await.get(&unique_key) {
                return Ok(response.clone());
            }
        }

        let res = self
            .coalescion
            .execute(unique_key.clone(), || async move {
                self.request::<types::CategoriesResponse>(
                    "/categories",
                    &[
                        ("api_key", &self.key),
                    ]
                ).await
            })
            .await
            .unwrap();

        if let Ok(resp) = &*res {
            self.categories
                .write()
                .await
                .insert(unique_key, resp.clone());
        }

        (*res).clone()
    }

    pub async fn trending(
        &self,
        limit: u32,
        offset: u64,
    ) -> Result<Arc<types::SearchResponse>, GiphyError> {
        let unique_key = format!("f-{limit}-{offset}");

        if self.featured.read().await.contains_key(&unique_key) {
            if let Some(response) = self.featured.write().await.get(&unique_key) {
                return Ok(response.clone());
            }
        }

        let limit_str = limit.to_string();
        let offset_str = offset.to_string();

        let res = self.coalescion.execute(unique_key.clone(), || async move {
            self.request::<types::SearchResponse>(
                "/trending",
                &[
                    ("api_key", &self.key),
                    ("limit", &limit_str),
                    ("offset", &offset_str),
                    ("rating", "g"),
                ]
            ).await
        })
        .await
        .unwrap();

        if let Ok(resp) = &*res {
            self.featured.write().await.insert(unique_key, resp.clone());
        }

        (*res).clone()
    }
}
