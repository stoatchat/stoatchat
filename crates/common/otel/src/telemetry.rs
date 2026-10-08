use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, Protocol, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

pub struct Telemetry {
    logger_provider: SdkLoggerProvider,
}

impl Telemetry {
    pub fn create(service: &'static str, version: &'static str) -> Telemetry {
        let resource = Resource::builder()
            .with_service_name(service)
            .with_attribute(KeyValue::new("service.version", version))
            .build();

        let log_exporter = LogExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .build()
            .expect("Failed to create log exporter");

        let logger_provider = SdkLoggerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(log_exporter)
            .build();

        let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider).with_filter(
            env_filter()
                .add_directive("hyper=off".parse().unwrap())
                .add_directive("tonic=off".parse().unwrap())
                .add_directive("h2=off".parse().unwrap())
                .add_directive("reqwest=off".parse().unwrap()),
        );

        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_thread_names(true)
            .with_filter(env_filter());

        tracing_subscriber::registry()
            .with(otel_layer)
            .with(fmt_layer)
            .init();

        debug!("hai i'm a log :3 (initialised OTLP successfully)");

        Telemetry { logger_provider }
    }
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        if let Err(e) = self.logger_provider.shutdown() {
            eprintln!("otel logger shutdown failed: {e}");
        }
    }
}
