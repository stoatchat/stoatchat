use lapin::types::{AMQPValue, FieldTable};
use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

struct HeaderExtractor<'a>(&'a FieldTable);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        match self.0.inner().get(key)? {
            AMQPValue::LongString(value) => std::str::from_utf8(value.as_bytes()).ok(),
            AMQPValue::ShortString(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn keys(&self) -> Vec<&str> {
        self.0.inner().keys().map(|key| key.as_str()).collect()
    }
}

struct HeaderInjector<'a>(&'a mut FieldTable);

impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.into(), AMQPValue::LongString(value.into()));
    }
}

/// Continue the trace the publisher stored in the message headers, if there is one.
/// Must be called before the span is entered.
pub fn set_parent_from_headers(span: &Span, headers: Option<&FieldTable>) {
    let Some(headers) = headers else {
        return;
    };

    let parent =
        global::get_text_map_propagator(|propagator| propagator.extract(&HeaderExtractor(headers)));

    // fails if the span isn't being exported, nothing to continue in that case
    let _ = span.set_parent(parent);
}

/// Store the current trace context in the message headers so the consumer can continue it
pub fn inject_headers(headers: &mut FieldTable) {
    let context = Span::current().context();

    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut HeaderInjector(headers))
    });
}

#[cfg(test)]
mod tests {
    use opentelemetry::{
        propagation::TextMapPropagator,
        trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
        Context,
    };
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    use super::*;

    const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn extracts_traceparent_from_headers() {
        let mut headers = FieldTable::default();
        headers.insert(
            "traceparent".into(),
            AMQPValue::LongString(TRACEPARENT.into()),
        );

        let context = TraceContextPropagator::new().extract(&HeaderExtractor(&headers));
        let span = context.span();
        let span_context = span.span_context();

        assert!(span_context.is_remote());
        assert_eq!(
            span_context.trace_id(),
            TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").unwrap()
        );
        assert_eq!(
            span_context.span_id(),
            SpanId::from_hex("00f067aa0ba902b7").unwrap()
        );
    }

    #[test]
    fn injects_traceparent_into_headers() {
        let context = Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").unwrap(),
            SpanId::from_hex("00f067aa0ba902b7").unwrap(),
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        ));

        let mut headers = FieldTable::default();
        TraceContextPropagator::new().inject_context(&context, &mut HeaderInjector(&mut headers));

        assert_eq!(HeaderExtractor(&headers).get("traceparent"), Some(TRACEPARENT));
    }
}
