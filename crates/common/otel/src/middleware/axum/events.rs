use std::time::{Duration, Instant};

use ::axum::{
    Json,
    extract::{MatchedPath, Request},
    http::{HeaderValue, header::CONTENT_LENGTH},
    middleware::Next,
    response::{IntoResponse, Response},
};
use opentelemetry::trace::TraceContextExt;
use serde::Serialize;
use tracing::{Instrument, Span, field::Empty, info_span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

use crate::{WideContextRepository, models::HttpRequestEvent};

#[derive(Serialize)]
struct ErrorBody<'a> {
    #[serde(flatten)]
    error: &'a revolt_result::Error,
    request_id: &'a str,
}

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

    fn finish_axum(&mut self, response: &mut Response, span: &Span) {
        self.status = response.status().as_u16();
        self.duration = self.start.elapsed();
        self.sampled = span.context().span().span_context().is_sampled();
        self.error = response.extensions_mut().remove::<revolt_result::Error>();
    }
}

pub async fn wide_events(mut request: Request, next: Next) -> Response {
    let context = WideContextRepository::default();
    request.extensions_mut().insert(context.clone());

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

    let mut response = next.run(request).instrument(span.clone()).await;
    event.finish_axum(&mut response, &span);
    span.record("http.response.status_code", event.status);

    if event.failed() {
        span.record("otel.status_code", "error");
    }

    if let Some(error) = &event.error {
        let (mut parts, _) = response.into_parts();
        parts.headers.remove(CONTENT_LENGTH);

        let body = Json(ErrorBody {
            error,
            request_id: &event.request_id,
        });

        response = Response::from_parts(parts, body.into_response().into_body());
    }

    if let Ok(request_id) = HeaderValue::from_str(&event.request_id) {
        response.headers_mut().insert("x-request-id", request_id);
    }

    span.in_scope(|| event.emit(context.take()));
    response
}
