use std::{fmt::Display, future::Future};

use lapin::message::Delivery;
use opentelemetry::trace::TraceContextExt;
use tracing::{Instrument, field::Empty, info_span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{WideContextRepository, amqp::set_parent_from_headers, models::AmqpMessageEvent};

/// Process a delivery inside a consumer span, continuing the publisher's trace,
/// and emit one wide event summarising it once the handler finishes
pub async fn wide_events<F, Fut, E>(
    consumer: &'static str,
    delivery: Delivery,
    handler: F,
) -> Result<(), E>
where
    F: FnOnce(Delivery, WideContextRepository) -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: Display,
{
    let context = WideContextRepository::default();

    let mut event = AmqpMessageEvent::from_delivery(consumer, &delivery);
    let span = info_span!(
        "amqp.message",
        otel.name = format!("process {}", event.routing_key),
        otel.kind = "consumer",
        consumer,
        messaging.system = "rabbitmq",
        messaging.operation.type = "process",
        messaging.destination.name = event.exchange,
        messaging.rabbitmq.destination.routing_key = event.routing_key,
        otel.status_code = Empty,
    );
    set_parent_from_headers(&span, delivery.properties.headers().as_ref());

    let result = handler(delivery, context.clone())
        .instrument(span.clone())
        .await;

    event.duration = event.start.elapsed();
    event.sampled = span.context().span().span_context().is_sampled();

    if let Err(error) = &result {
        event.error = Some(format!("{error:#}"));
        span.record("otel.status_code", "error");
    }

    span.in_scope(|| event.emit(context.take()));
    result
}
