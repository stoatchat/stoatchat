use ::axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument, field::Empty, info_span};
use uuid::Uuid;

pub async fn wide_events(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| path.clone());

    let span = info_span!(
        "http.request",
        otel.name = format!("{method} {route}"),
        otel.kind = "server",
        request_id,
        http.request.method = method,
        http.route = route,
        url.path = path,
        http.response.status_code = Empty,
        otel.status_code = Empty,
    );

    let response = next.run(request).instrument(span.clone()).await;
    let status = response.status().as_u16();
    span.record("http.response.status_code", status);

    if status >= 500 {
        span.record("otel.status_code", "error");
    }

    response
}
