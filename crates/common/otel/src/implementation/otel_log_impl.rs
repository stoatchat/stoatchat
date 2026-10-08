use std::sync::OnceLock;

use opentelemetry::{
    Key,
    logs::{AnyValue, LogRecord, Logger, Severity},
};
use opentelemetry_sdk::logs::SdkLogger;
use tracing::Level;

use crate::models::HttpRequestEvent;

pub(crate) static WIDE_LOGGER: OnceLock<SdkLogger> = OnceLock::new();

impl HttpRequestEvent {
    pub(crate) fn emit(self, attributes: Vec<(Key, AnyValue)>) {
        let level = self.level();
        let message = self.summary();
        let outcome = if self.rejected() { "error" } else { "success" };
        let duration_ms = self.duration.as_millis() as i64;

        if let Some(logger) = WIDE_LOGGER.get() {
            let (severity, severity_text) = match level {
                Level::ERROR => (Severity::Error, "ERROR"),
                Level::WARN => (Severity::Warn, "WARN"),
                _ => (Severity::Info, "INFO"),
            };

            let mut record = logger.create_log_record();
            record.set_event_name("http.request");
            record.set_severity_number(severity);
            record.set_severity_text(severity_text);
            record.set_body(message.clone().into());
            record.add_attribute("request_id", self.request_id);
            record.add_attribute("method", self.method);
            record.add_attribute("path", self.path);
            record.add_attribute("route", self.route);
            record.add_attribute("status_code", self.status as i64);
            record.add_attribute("outcome", outcome);
            record.add_attribute("duration_ms", duration_ms);

            if let Some(error) = self.error {
                let kind: &'static str = (&error.error_type).into();
                record.add_attribute("error.type", kind);
                record.add_attribute("error.message", format!("{:?}", error.error_type));

                if let Some(location) = error.location {
                    record.add_attribute("error.location", location);
                }
            }

            record.add_attributes(attributes);
            logger.emit(record);
        }

        let status = self.status;
        match level {
            Level::ERROR => {
                tracing::error!(target: "http_request", status, duration_ms, "{message}")
            }
            Level::WARN => {
                tracing::warn!(target: "http_request", status, duration_ms, "{message}")
            }
            _ => tracing::info!(target: "http_request", status, duration_ms, "{message}"),
        }
    }
}
