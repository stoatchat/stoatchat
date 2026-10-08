use std::net::{Ipv4Addr, SocketAddr};

use axum::{
    middleware::{from_fn, from_fn_with_state},
    Router,
};

use axum_macros::FromRef;
use revolt_database::{Database, DatabaseInfo};
use revolt_ratelimits::axum as ratelimiter;
use tokio::{
    net::TcpListener,
    signal::unix::{signal, SignalKind},
};
use tower_http::catch_panic::CatchPanicLayer;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable as ScalarServable};
use tracing::info;

mod api;
pub mod clamav;
pub mod exif;
pub mod metadata;
pub mod mime_type;
mod ratelimits;
mod utils;

#[derive(FromRef, Clone)]
struct AppState {
    database: Database,
    ratelimit_storage: ratelimiter::RatelimitStorage,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let _telemetry = stoat_otel::init(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!(version = env!("CARGO_PKG_VERSION"), awawa = true, "Starting the Stoat file server!");

    revolt_config::config().await;
    clamav::init().await;

    #[derive(OpenApi)]
    #[openapi(
        modifiers(&SecurityAddon),
        paths(
            api::root,
            api::upload_file,
            api::fetch_preview,
            api::fetch_file
        ),
        components(
            schemas(
                revolt_result::Error,
                revolt_result::ErrorType,
                api::RootResponse,
                api::Tag,
                api::UploadPayload,
                api::UploadResponse
            )
        ),
        tags(
            (name = "Files", description = "File uploads API")
        )
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "bot_token",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Bot-Token"))),
                );
                components.add_security_scheme(
                    "session_token",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Session-Token"))),
                );
            }
        }
    }

    let db = DatabaseInfo::Auto.connect().await.unwrap();
    let ratelimits = ratelimiter::RatelimitStorage::new(ratelimits::AutumnRatelimits);

    let state = AppState {
        database: db,
        ratelimit_storage: ratelimits,
    };

    let app = Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .nest("/", api::router().await)
        .nest("/", ratelimiter::routes())
        .layer(from_fn_with_state(
            state.clone(),
            ratelimiter::ratelimit_middleware,
        ))
        .layer(CatchPanicLayer::custom(stoat_otel::middleware::axum::panic_response))
        .layer(from_fn(stoat_otel::middleware::axum::wide_events))
        .with_state(state);

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 14704));
    let listener = TcpListener::bind(&address).await?;
    info!(address = %listener.local_addr()?, "Listening for requests 🍂");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let mut terminate =
            signal(SignalKind::terminate()).expect("sigterm handler");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }

        info!("Service going offline, bye bye");
    })
    .await?;

    Ok(())
}
