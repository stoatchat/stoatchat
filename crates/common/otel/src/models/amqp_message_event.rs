use std::time::{Duration, Instant};

use lapin::message::Delivery;
use tracing::Level;

const SLOW_MESSAGE_THRESHOLD: Duration = Duration::from_secs(1);

pub(crate) struct AmqpMessageEvent {
    pub consumer: &'static str,
    pub exchange: String,
    pub routing_key: String,
    pub redelivered: bool,
    pub start: Instant,
    pub duration: Duration,
    pub sampled: bool,
    pub error: Option<String>,
}

impl AmqpMessageEvent {
    pub(crate) fn from_delivery(consumer: &'static str, delivery: &Delivery) -> Self {
        AmqpMessageEvent {
            consumer,
            exchange: delivery.exchange.to_string(),
            routing_key: delivery.routing_key.to_string(),
            redelivered: delivery.redelivered,
            start: Instant::now(),
            duration: Duration::ZERO,
            sampled: false,
            error: None,
        }
    }

    pub(crate) fn failed(&self) -> bool {
        self.error.is_some()
    }

    fn slow(&self) -> bool {
        self.duration > SLOW_MESSAGE_THRESHOLD
    }

    pub(crate) fn level(&self) -> Level {
        if self.failed() {
            Level::ERROR
        } else if self.slow() {
            Level::WARN
        } else {
            Level::INFO
        }
    }

    pub(crate) fn summary(&self) -> String {
        let outcome = if self.failed() { "ERROR" } else { "OK" };
        let slow = if self.slow() { "[SLOW] " } else { "" };

        let Self {
            consumer,
            routing_key,
            ..
        } = self;

        format!("{slow}{consumer} {routing_key} ({outcome})")
    }
}
