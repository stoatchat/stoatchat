use crate::AppState;
use axum::{
    http::Method,
    routing::{get, Router},
};
use tower_http::cors::{AllowHeaders, Any, CorsLayer};

pub mod categories;
pub mod root;
pub mod search;
pub mod trending;

pub fn router() -> Router<AppState> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_headers(AllowHeaders::mirror_request())
        .expose_headers(vec![
            "X-RateLimit-Limit".try_into().unwrap(),
            "X-RateLimit-Bucket".try_into().unwrap(),
            "X-RateLimit-Remaining".try_into().unwrap(),
            "X-RateLimit-Reset-After".try_into().unwrap(),
        ])
        .allow_origin(Any);

    Router::new()
        .route("/", get(root::root))
        .route("/categories", get(categories::categories))
        .route("/search", get(search::search))
        .route("/trending", get(trending::trending))
        .layer(cors)
}
