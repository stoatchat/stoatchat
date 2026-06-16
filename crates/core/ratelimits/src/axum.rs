use async_trait::async_trait;
use axum::{
    Json, RequestPartsExt, Router,
    body::Body,
    extract::{FromRef, FromRequestParts, State},
    http::{HeaderValue, Request, StatusCode, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
};
use revolt_database::{Database, User, util::ip::axum::to_real_ip};

use crate::ratelimiter::{RatelimitInformation, Ratelimiter, RequestKind};

#[derive(Clone, Copy)]
pub struct AxumRequestKind;

impl RequestKind for AxumRequestKind {
    type R<'a> = Parts;
}

pub type RatelimitStorage = crate::ratelimiter::RatelimitStorage<AxumRequestKind>;

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ratelimiter
where
    Database: FromRef<S>,
    RatelimitStorage: FromRef<S>,
{
    type Rejection = Json<Ratelimiter>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if parts
            .extensions
            .get::<Result<Ratelimiter, Json<Ratelimiter>>>()
            .is_none()
        {
            let storage = RatelimitStorage::from_ref(state);

            let identifier = if let Ok(user) = parts.extract_with_state::<User, _>(state).await {
                user.id
            } else {
                to_real_ip(parts).await
            };

            let (bucket, resource) = storage.resolver.resolve_bucket(parts);
            let limit = storage.resolver.resolve_bucket_limit(bucket);

            let ratelimiter =
                Ratelimiter::from(&storage.map, &identifier, limit, (bucket, resource));

            parts.extensions.insert(ratelimiter.map_err(Json));
        };

        *parts
            .extensions
            .get::<Result<Ratelimiter, Json<Ratelimiter>>>()
            .unwrap()
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for RatelimitInformation
where
    Database: FromRef<S>,
    RatelimitStorage: FromRef<S>,
{
    type Rejection = Json<RatelimitInformation>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if parts
            .extensions
            .get::<Result<Ratelimiter, Json<Ratelimiter>>>()
            .is_none()
        {
            let ratelimiter = parts.extract_with_state::<Ratelimiter, S>(state).await;

            parts.extensions.insert(ratelimiter);
        };

        let ratelimiter = *parts
            .extensions
            .get::<Result<Ratelimiter, Json<Ratelimiter>>>()
            .unwrap();

        match ratelimiter {
            Ok(ratelimter) => Ok(RatelimitInformation::Success(ratelimter)),
            Err(ratelimiter) => Err(Json(RatelimitInformation::Failure {
                retry_after: ratelimiter.reset,
            })),
        }
    }
}

pub async fn ratelimit_middleware(
    State(database): State<Database>,
    State(ratelimit_storage): State<RatelimitStorage>,
    request: Request<Body>,
    next: Next,
) -> Response {
    #[derive(axum::extract::FromRef)]
    struct TempState {
        database: Database,
        ratelimit_storage: RatelimitStorage,
    }

    let state = TempState {
        database,
        ratelimit_storage,
    };

    let (mut parts, body) = request.into_parts();

    let res = Ratelimiter::from_request_parts(&mut parts, &state).await;

    let (Ok(ratelimiter) | Err(Json(ratelimiter))) = &res;

    let mut response = if res.is_ok() {
        let request = Request::from_parts(parts, body);

        next.run(request).await
    } else {
        let ratelimit_info = RatelimitInformation::from_request_parts(&mut parts, &state).await;

        ratelimit_info.map(Json).into_response()
    };

    let Ratelimiter {
        key,
        limit,
        remaining,
        reset,
    } = ratelimiter;

    let headers = response.headers_mut();

    headers.insert(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&limit.to_string()).unwrap(),
    );
    headers.insert(
        "X-RateLimit-Bucket",
        HeaderValue::from_str(&key.to_string()).unwrap(),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&remaining.to_string()).unwrap(),
    );
    headers.insert(
        "X-RateLimit-Reset-After",
        HeaderValue::from_str(&reset.to_string()).unwrap(),
    );

    if res.is_err() {
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    };

    response
}

async fn ratelimit_info(info: RatelimitInformation) -> Json<RatelimitInformation> {
    Json(info)
}

pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S>
where
    Database: FromRef<S>,
    RatelimitStorage: FromRef<S>,
{
    Router::new().route("/ratelimit", get(ratelimit_info))
}
