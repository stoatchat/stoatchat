use std::time::{Duration, Instant};

use ::axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use opentelemetry::trace::TraceContextExt;
use tracing::{Instrument, Span, field::Empty, info_span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

use crate::{WideContextRepository, models::HttpRequestEvent};

impl HttpRequestEvent {
    fn from_axum_request(request: &Request) -> Self {
        let path = request.uri().path().to_owned();

        HttpRequestEvent {
            request_id: request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            method: request.method().to_string(),
            route: request
                .extensions()
                .get::<MatchedPath>()
                .map(|matched| matched.as_str().to_owned())
                .unwrap_or_else(|| path.clone()),
            path,
            start: Instant::now(),
            status: 0,
            duration: Duration::ZERO,
            sampled: false,
            error: None,
        }
    }

    fn finish_axum(&mut self, response: &Response, span: &Span) {
        self.status = response.status().as_u16();
        self.duration = self.start.elapsed();
        self.sampled = span.context().span().span_context().is_sampled();
        self.error = response.extensions().get::<revolt_result::Error>().cloned();
    }
}

pub async fn wide_events(request: Request, next: Next) -> Response {
    let context = WideContextRepository::default();

    let mut event = HttpRequestEvent::from_axum_request(&request);
    let span = info_span!(
        "http.request",
        otel.name = format!("{} {}", event.method, event.route),
        otel.kind = "server",
        request_id = event.request_id,
        http.request.method = event.method,
        http.route = event.route,
        url.path = event.path,
        http.response.status_code = Empty,
        otel.status_code = Empty,
    );

    let response = next.run(request).instrument(span.clone()).await;
    event.finish_axum(&response, &span);
    span.record("http.response.status_code", event.status);

    if event.failed() {
        span.record("otel.status_code", "error");
    }

    span.in_scope(|| event.emit(context.take()));
    response
}
