use std::net::{Ipv4Addr, SocketAddr};

use axum::{middleware::from_fn_with_state, Router};

use axum_macros::FromRef;
use revolt_database::{Database, DatabaseInfo};
use revolt_ratelimits::axum as ratelimiter;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable as ScalarServable};

mod api;
pub mod clamav;
pub mod exif;
pub mod metadata;
pub mod mime_type;
mod ratelimits;

#[derive(FromRef, Clone)]
struct AppState {
    database: Database,
    ratelimit_storage: ratelimiter::RatelimitStorage,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let logger_provider = init_logs();

    let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());

    let otel_layer = otel_layer.with_filter(filter_otel);

    let filter_fmt = EnvFilter::new("info");

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(filter_fmt);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    revolt_config::configure!(files);
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
            // (name = "Files", description = "File uploads API")
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
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 14704));
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, app.into_make_service()).await?;

    if let Err(e) = logger_provider.shutdown() {
        panic!("logger provider failed to shut down");
    }

    Ok(())
}

use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::KeyValue;
use opentelemetry::{global, InstrumentationScope};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::error::Error;
use std::sync::OnceLock;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| {
            Resource::builder()
                .with_service_name("basic-otlp-example-grpc")
                .build()
        })
        .clone()
}

fn init_logs() -> SdkLoggerProvider {
    let exporter = LogExporter::builder()
        .with_http()
        .with_endpoint("http://localhost:19428/insert/opentelemetry/v1/logs")
        .with_protocol(Protocol::HttpBinary)
        .build()
        .expect("Failed to create log exporter");

    SdkLoggerProvider::builder()
        .with_resource(get_resource())
        .with_batch_exporter(exporter)
        .build()
}
