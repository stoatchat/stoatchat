use opentelemetry::{
    Key,
    logs::{AnyValue, LogRecord, Logger, Severity},
};
use tracing::Level;

use super::WIDE_LOGGER;
use crate::models::AmqpMessageEvent;

impl AmqpMessageEvent {
    pub(crate) fn emit(self, attributes: Vec<(Key, AnyValue)>) {
        let level = self.level();
        let message = self.summary();
        let outcome = if self.failed() { "error" } else { "success" };
        let duration_ms = self.duration.as_millis() as i64;

        let logger = WIDE_LOGGER
            .get()
            .filter(|_| self.sampled || level == Level::ERROR);

        if let Some(logger) = logger {
            let (severity, severity_text) = match level {
                Level::ERROR => (Severity::Error, "ERROR"),
                Level::WARN => (Severity::Warn, "WARN"),
                _ => (Severity::Info, "INFO"),
            };

            let mut record = logger.create_log_record();
            record.set_event_name("amqp.message");
            record.set_severity_number(severity);
            record.set_severity_text(severity_text);
            record.set_body(message.clone().into());
            record.add_attribute("consumer", self.consumer);
            record.add_attribute("exchange", self.exchange);
            record.add_attribute("routing_key", self.routing_key);
            record.add_attribute("redelivered", self.redelivered);
            record.add_attribute("outcome", outcome);
            record.add_attribute("duration_ms", duration_ms);

            if let Some(error) = self.error {
                record.add_attribute("error.message", error);
            }

            record.add_attributes(attributes);
            logger.emit(record);
        }

        match level {
            Level::ERROR => tracing::error!(target: "amqp_message", duration_ms, "{message}"),
            Level::WARN => tracing::warn!(target: "amqp_message", duration_ms, "{message}"),
            _ => tracing::info!(target: "amqp_message", duration_ms, "{message}"),
        }
    }
}
