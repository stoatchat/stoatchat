use opentelemetry::KeyValue;
#[cfg(feature = "axum")]
use opentelemetry::logs::LoggerProvider;
use opentelemetry::trace::TracerProvider;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

#[cfg(feature = "axum")]
use crate::implementation::WIDE_LOGGER;

pub struct Telemetry {
    logger_provider: SdkLoggerProvider,
    tracer_provider: SdkTracerProvider,
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
            .with_resource(resource.clone())
            .with_batch_exporter(log_exporter)
            .build();

        let span_exporter = SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .build()
            .expect("Failed to create span exporter");

        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(span_exporter)
            .build();

        #[cfg(feature = "axum")]
        let _ = WIDE_LOGGER.set(logger_provider.logger("stoat_otel::wide"));

        let otel_layer =
            OpenTelemetryTracingBridge::new(&logger_provider).with_filter(export_filter());

        let trace_layer = tracing_opentelemetry::layer()
            .with_tracer(tracer_provider.tracer("stoat_otel"))
            .with_filter(export_filter());

        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_thread_names(true)
            .with_filter(env_filter());

        tracing_subscriber::registry()
            .with(otel_layer)
            .with(trace_layer)
            .with(fmt_layer)
            .init();

        debug!("hai i'm a log :3 (initialised OTLP successfully)");

        Telemetry {
            logger_provider,
            tracer_provider,
        }
    }
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

fn export_filter() -> EnvFilter {
    env_filter()
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap())
        .add_directive("aws_smithy_runtime=off".parse().unwrap())
        .add_directive("aws_sdk_s3=off".parse().unwrap())
        .add_directive("http_request=off".parse().unwrap())
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        if let Err(e) = self.logger_provider.shutdown() {
            eprintln!("otel logger shutdown failed: {e}");
        }

        if let Err(e) = self.tracer_provider.shutdown() {
            eprintln!("otel tracer shutdown failed: {e}");
        }
    }
}
