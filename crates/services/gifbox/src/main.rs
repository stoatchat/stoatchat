use std::net::{Ipv4Addr, SocketAddr};

use axum::{extract::FromRef, middleware::from_fn_with_state, Router};

use revolt_config::config;
use revolt_database::{Database, DatabaseInfo};
use revolt_ratelimits::axum as ratelimiter;
use tokio::net::TcpListener;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable as ScalarServable};

use crate::tenor::Tenor;

mod ratelimits;
mod routes;
mod tenor;
mod types;

#[derive(Clone, FromRef)]
struct AppState {
    pub database: Database,
    pub tenor: Tenor,
    pub ratelimit_storage: ratelimiter::RatelimitStorage,
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();

        components.add_security_scheme(
            "User Token",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(
                "X-Session-Token".to_string(),
            ))),
        );

        components.add_security_scheme(
            "Bot Token",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Bot-Token".to_string()))),
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Configure logging and environment
    revolt_config::configure!(gifbox);

    // Configure API schema
    #[derive(OpenApi)]
    #[openapi(
        modifiers(&SecurityAddon),
        paths(
            routes::categories::categories,
            routes::root::root,
            routes::search::search,
            routes::trending::trending,
        ),
        tags(
            (name = "Misc", description = "Misc routes for microservice."),
            (name = "GIFs", description = "All routes for requesting GIFs from tenor.")
        ),
        components(
            schemas(
                revolt_result::Error,
                revolt_result::ErrorType,
                types::MediaResult,
                types::MediaObject,
            )
        ),
    )]
    struct ApiDoc;

    let config = config().await;

    let database = DatabaseInfo::Auto
        .connect()
        .await
        .expect("Unable to connect to database");

    let tenor = tenor::Tenor::new(&config.api.security.tenor_key);

    let ratelimit_storage = ratelimiter::RatelimitStorage::new(ratelimits::GifboxRatelimits);

    let state = AppState {
        database,
        tenor,
        ratelimit_storage,
    };

    // Configure Axum and router
    let app = Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .nest("/", routes::router())
        .layer(from_fn_with_state(
            state.clone(),
            ratelimiter::ratelimit_middleware,
        ))
        .with_state(state);

    // Configure TCP listener and bind
    tracing::info!("Listening on 0.0.0.0:14706");
    tracing::info!("Play around with the API: http://localhost:14706/scalar");
    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 14706));
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, app.into_make_service()).await
}
