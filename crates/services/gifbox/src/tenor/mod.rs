//! Internal Tenor API wrapper

use std::{sync::Arc, time::Duration};

use lru_time_cache::LruCache;
use reqwest::Client;
use revolt_coalesced::{CoalescionService, CoalescionServiceConfig};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

pub mod types;

const TENOR_API_BASE_URL: &str = "https://tenor.googleapis.com/v2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TenorError {
    HttpError,
}

#[derive(Clone)]
pub struct Tenor {
    pub key: Arc<str>,
    pub client: Client,
    pub coalescion: CoalescionService<String>,
    pub cache: Arc<RwLock<LruCache<String, Arc<types::PaginatedMediaResponse>>>>,

    pub categories: Arc<RwLock<LruCache<String, Arc<types::CategoriesResponse>>>>,
    pub featured: Arc<RwLock<LruCache<String, Arc<types::PaginatedMediaResponse>>>>,
}

impl Tenor {
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

    pub async fn request<T: DeserializeOwned>(&self, path: &str, query: &[Option<(&str, &str)>]) -> Result<Arc<T>, TenorError> {
        let response = self
            .client
            .get(format!("{TENOR_API_BASE_URL}{path}"))
            .query(query)
            .send()
            .await
            .inspect_err(|e| {
                revolt_config::capture_error(e);
            })
            .map_err(|_| TenorError::HttpError)?;

        let text = response.text().await.map_err(|e| {
            revolt_config::capture_error(&e);
            TenorError::HttpError
        })?;

        Ok(Arc::new(serde_json::from_str(&text).unwrap()))
    }

    pub async fn search(
        &self,
        query: &str,
        locale: &str,
        limit: u32,
        is_category: bool,
        position: &str,
    ) -> Result<Arc<types::PaginatedMediaResponse>, TenorError> {
        let unique_key = format!("s:{query}:{locale}:{is_category}:{position}");

        if self.cache.read().await.contains_key(&unique_key) {
            if let Some(response) = self.cache.write().await.get(&unique_key) {
                return Ok(response.clone());
            }
        }

        let res = self.coalescion.execute(unique_key.clone(), || async move {
            self.request::<types::PaginatedMediaResponse>(
                "/search",
                &[
                    Some(("key", &self.key)),
                    Some(("q", query)),
                    Some(("client_key", "Gifbox")),
                    Some(("media_filter", "webm,tinywebm")),
                    Some(("locale", locale)),
                    Some(("contentfilter", "high")),
                    Some(("limit", &limit.to_string())),
                    position.is_empty().then_some(("pos", position)),
                    is_category.then_some(("component", "categories"))
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
        locale: &str,
    ) -> Result<Arc<types::CategoriesResponse>, TenorError> {
        let unique_key = format!("c-{locale}");

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
                        Some(("key", &self.key)),
                        Some(("client_key", "Gifbox")),
                        Some(("locale", locale)),
                        Some(("contentfilter", "high")),
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

    pub async fn featured(
        &self,
        locale: &str,
        limit: u32,
        position: &str,
    ) -> Result<Arc<types::PaginatedMediaResponse>, TenorError> {
        let unique_key = format!("f-{locale}-{limit}-{position}");

        if self.categories.read().await.contains_key(&unique_key) {
            if let Some(response) = self.featured.write().await.get(&unique_key) {
                return Ok(response.clone());
            }
        }

        let res = self.coalescion.execute(unique_key.clone(), || async move {
            self.request::<types::PaginatedMediaResponse>(
                "/featured",
                &[
                    Some(("key", &self.key)),
                    Some(("client_key", "Gifbox")),
                    Some(("media_filter", "webm,tinywebm")),
                    Some(("locale", locale)),
                    Some(("contentfilter", "high")),
                    Some(("limit", &limit.to_string())),
                    position.is_empty().then_some(("pos", position)),
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
