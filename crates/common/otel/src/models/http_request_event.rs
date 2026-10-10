use std::time::{Duration, Instant};

use tracing::Level;

const SLOW_REQUEST_THRESHOLD: Duration = Duration::from_millis(500);

pub(crate) struct HttpRequestEvent {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub route: String,
    pub start: Instant,
    pub status: u16,
    pub duration: Duration,
    pub sampled: bool,
    pub error: Option<revolt_result::Error>,
}

impl HttpRequestEvent {
    pub(crate) fn failed(&self) -> bool {
        self.status >= 500
    }

    pub(crate) fn rejected(&self) -> bool {
        self.status >= 400
    }

    fn slow(&self) -> bool {
        self.duration > SLOW_REQUEST_THRESHOLD
    }

    pub(crate) fn level(&self) -> Level {
        if self.failed() {
            Level::ERROR
        } else if self.rejected() || self.slow() {
            Level::WARN
        } else {
            Level::INFO
        }
    }

    pub(crate) fn summary(&self) -> String {
        let outcome = match &self.error {
            Some(error) => {
                let kind: &str = (&error.error_type).into();
                format!("ERROR {kind}")
            }
            None if self.rejected() => "ERROR".to_owned(),
            None => "OK".to_owned(),
        };
        let slow = if self.slow() { "[SLOW] " } else { "" };

        let Self { method, route, .. } = self;

        format!("{slow}{method} {route} ({outcome})")
    }
}
