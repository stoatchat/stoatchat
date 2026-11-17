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
