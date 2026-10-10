#[cfg(feature = "axum")]
mod implementation;
pub mod middleware;
#[cfg(feature = "axum")]
mod models;
#[cfg(feature = "axum")]
mod repositories;
mod telemetry;

#[cfg(feature = "axum")]
pub use repositories::*;
pub use telemetry::Telemetry;

pub fn init(service: &'static str, version: &'static str) -> Telemetry {
    Telemetry::create(service, version)
}
