mod telemetry;

pub use telemetry::Telemetry;

pub fn init(service: &'static str, version: &'static str) -> Telemetry {
    Telemetry::create(service, version)
}
