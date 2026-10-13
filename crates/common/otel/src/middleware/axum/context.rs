use std::convert::Infallible;

use ::axum::{async_trait, extract::FromRequestParts, http::request::Parts};

use crate::WideContextRepository;

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for WideContextRepository {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<WideContextRepository>()
            .cloned()
            .unwrap_or_default())
    }
}
