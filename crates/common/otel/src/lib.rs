#[cfg(feature = "amqp")]
pub mod amqp;
#[cfg(any(feature = "axum", feature = "amqp"))]
mod implementation;
pub mod middleware;
#[cfg(any(feature = "axum", feature = "amqp"))]
mod models;
#[cfg(any(feature = "axum", feature = "amqp"))]
mod repositories;
mod telemetry;

#[cfg(any(feature = "axum", feature = "amqp"))]
pub use repositories::*;
pub use telemetry::Telemetry;

pub fn init(service: &'static str, version: &'static str) -> Telemetry {
    Telemetry::create(service, version)
}

/// Whether to attach verbose detail to spans, enabled with `EXTRA_TRACING=1`
pub fn extra_tracing() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("EXTRA_TRACING").is_ok_and(|value| value == "1"))
}
