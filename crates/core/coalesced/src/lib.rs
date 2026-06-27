//! # Coalesced
//!
//! Coalescion service to group, caching and queue duplicate actions.
//! useful for deduplicating web requests, database lookups and other similar resource
//! intensive or rate-limited actions.
//!
//! ## Features
//! - `tokio`: Uses tokio for the async backend, this is currently the only backend.
//! - `queue`: Whether to support queueing requests to only allow X amount of actions running at once.
//! - `cache`: Whether to cache the actions results for future actions with the same id, uses an LRU cache internally.
//!
//! [`CoalescionService`] uses both [`Arc`] and [`RwLock`] internally and can be cheaply cloned to
//! use in your codebase.
//!
//! It is common practice to wrap the service and in your own which delegates the executions to ensure all ids are tracked in one location across your codebase.
//!
//! All values are stored using [`Any`] and must be [`'static`] + [`Send`] + [`Sync`], if there is an id mismatch
//! and a type is wrong the library will return an error, values returned from the service are also
//! wrapped in an [`Arc`] as they are shared to each duplicate action.
//!
//! ## Example:
//! ```rs
//! use revolt_coalesced::CoalescionService;
//!
//! let service = CoalescionService::new();
//!
//! let user_id = "my_user_id";
//! let user = service.execute(user_id, || async move {
//!     database.fetch_user(user_id).await.unwrap()
//! }).await;
//! ```

mod config;
mod error;
mod service;

pub use config::CoalescionServiceConfig;
pub use error::Error;
pub use service::CoalescionService;
