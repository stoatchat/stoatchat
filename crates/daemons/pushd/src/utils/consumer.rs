use std::{
    collections::HashMap,
    future::{ready, Future},
    pin::Pin,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use lapin::{
    message::{Delivery, DeliveryResult},
    options::BasicPublishOptions,
    BasicProperties, Channel, Connection, ConsumerDelegate, Error as AMQPError,
};
use revolt_database::{
    events::rabbit::{PayloadKind, PayloadToService},
    Database, Session,
};
use stoat_otel::WideContextRepository;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[async_trait]
pub trait Consumer: Clone + Send + Sync + 'static {
    async fn create(
        db: Database,
        connection: Arc<Connection>,
        channel: Arc<Channel>,
    ) -> Self;
    fn channel(&self) -> &Arc<Channel>;
    async fn consume(&self, delivery: Delivery, context: &WideContextRepository) -> Result<()>;

    async fn publish_message_with_options(
        &self,
        payload: &[u8],
        exchange: &str,
        routing_key: &str,
        options: BasicPublishOptions,
        properties: BasicProperties,
    ) -> Result<(), AMQPError> {
        let span = info_span!(
            "amqp.publish",
            otel.name = format!("publish {routing_key}"),
            otel.kind = "producer",
            messaging.system = "rabbitmq",
            messaging.operation.type = "send",
            messaging.destination.name = exchange,
            messaging.rabbitmq.destination.routing_key = routing_key,
        );

        async move {
            let channel = self.channel();

            // injected inside the producer span so the consumer's span hangs off it
            let mut headers = properties.headers().clone().unwrap_or_default();
            stoat_otel::amqp::inject_headers(&mut headers);
            let properties = properties.with_headers(headers);

            channel
                .basic_publish(
                    exchange.into(),
                    routing_key.into(),
                    options,
                    payload,
                    properties,
                )
                .await?;
            debug!(routing_key, "Sent message to queue");

            Ok(())
        }
        .instrument(span)
        .await
    }

    async fn publish_message(
        &self,
        payload: &[u8],
        exchange: &str,
        routing_key: &str,
    ) -> Result<(), AMQPError> {
        self.publish_message_with_options(
            payload,
            exchange,
            routing_key,
            BasicPublishOptions::default(),
            BasicProperties::default(),
        )
        .await
    }

    /// Queue a push for the session on its provider's outbound queue.
    /// Returns whether anything was queued, sessions without a subscription are skipped.
    async fn publish_push(&self, notification: PayloadKind, session: Session) -> Result<bool> {
        let Some(sub) = session.subscription else {
            return Ok(false);
        };

        let config = revolt_config::config().await;

        let mut sendable = PayloadToService {
            notification,
            token: sub.auth,
            user_id: session.user_id,
            session_id: session.id,
            extras: HashMap::new(),
        };

        let routing_key = match sub.endpoint.as_str() {
            "apn" => &config.pushd.apn.queue,
            "fcm" => &config.pushd.fcm.queue,
            endpoint => {
                sendable.extras.insert("p256dh".to_string(), sub.p256dh);
                sendable
                    .extras
                    .insert("endpoint".to_string(), endpoint.to_string());

                &config.pushd.vapid.queue
            }
        };

        let payload = serde_json::to_string(&sendable)?;
        self.publish_message(payload.as_bytes(), &config.pushd.exchange, routing_key)
            .await?;

        Ok(true)
    }
}

/// Short name for the kind of notification, used in spans and wide events
pub fn notification_kind(notification: &PayloadKind) -> &'static str {
    match notification {
        PayloadKind::MessageNotification(_) => "message",
        PayloadKind::FRAccepted(_) => "fr_accepted",
        PayloadKind::FRReceived(_) => "fr_received",
        PayloadKind::BadgeUpdate(_) => "badge_update",
        PayloadKind::Generic(_) => "generic",
        PayloadKind::DmCallStartEnd(_) => "dm_call",
    }
}

/// Attach the target of an outbound push to the wide event and the current span
pub fn record_push_target(
    context: &WideContextRepository,
    provider: &'static str,
    payload: &PayloadToService,
) {
    let kind = notification_kind(&payload.notification);

    context.add("push.provider", provider);
    context.add("push.kind", kind);
    context.add("user_id", payload.user_id.clone());
    context.add("session_id", payload.session_id.clone());

    let span = tracing::Span::current();
    span.set_attribute("push.provider", provider);
    span.set_attribute("push.kind", kind);
    span.set_attribute("user_id", payload.user_id.clone());
    span.set_attribute("session_id", payload.session_id.clone());
}

pub struct Delegate<C: Consumer>(pub C);

fn consumer_name<C>() -> &'static str {
    let name = std::any::type_name::<C>();
    name.rsplit("::").next().unwrap_or(name)
}

impl<C: Consumer> ConsumerDelegate for Delegate<C> {
    fn on_new_delivery(
        &self,
        delivery: DeliveryResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let consumer_name = consumer_name::<C>();

        match delivery {
            Ok(Some(delivery)) => {
                let consumer = self.0.clone();

                Box::pin(async move {
                    let _ = stoat_otel::middleware::amqp::wide_events(
                        consumer_name,
                        delivery,
                        |delivery, context| async move { consumer.consume(delivery, &context).await },
                    )
                    .await;
                })
            }
            Ok(None) => Box::pin(ready(())),
            Err(error) => Box::pin(async move {
                error!(consumer = consumer_name, error = ?error, "Received bad delivery")
            }),
        }
    }
}
