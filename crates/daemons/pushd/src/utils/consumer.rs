use std::{
    collections::{HashMap, HashSet},
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
use opentelemetry::KeyValue;
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
            record_skip(&session.user_id, Some(&session.id), "no_subscription");
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
        PayloadKind::SystemNotice(_) => "system_notice",
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

    if let PayloadKind::MessageNotification(push) = &payload.notification {
        context.add("message_id", push.message.id.clone());
        context.add("channel_id", push.message.channel.clone());
        span.set_attribute("message_id", push.message.id.clone());
        span.set_attribute("channel_id", push.message.channel.clone());
    }
}

/// Record why a user wasn't sent a push as an event on the current span, only when `EXTRA_TRACING=1`.
/// Goes straight to the trace rather than through tracing, so it doesn't flood the logs
pub fn record_skip(user_id: &str, session_id: Option<&str>, reason: &'static str) {
    if !stoat_otel::extra_tracing() {
        return;
    }

    let mut attributes = vec![
        KeyValue::new("user_id", user_id.to_string()),
        KeyValue::new("reason", reason),
    ];

    if let Some(session_id) = session_id {
        attributes.push(KeyValue::new("session_id", session_id.to_string()));
    }

    tracing::Span::current().add_event("push.skipped", attributes);
}

/// Record every user that was skipped for the same reason, only when `EXTRA_TRACING=1`
pub fn record_skips<'a>(user_ids: impl IntoIterator<Item = &'a String>, reason: &'static str) {
    if !stoat_otel::extra_tracing() {
        return;
    }

    for user_id in user_ids {
        record_skip(user_id, None, reason);
    }
}

/// Record users that had no push-capable sessions, only when `EXTRA_TRACING=1`
pub fn record_users_without_sessions(users: &[String], sessions: &[Session]) {
    if !stoat_otel::extra_tracing() {
        return;
    }

    let with_sessions: HashSet<&str> = sessions
        .iter()
        .map(|session| session.user_id.as_str())
        .collect();

    record_skips(
        users
            .iter()
            .filter(|user_id| !with_sessions.contains(user_id.as_str())),
        "no_sessions",
    );
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
                        |delivery, context| async move {
                            let result = consumer.consume(delivery, &context).await;

                            // deliveries are consumed with no_ack, so a failure here means the message is gone for good
                            if let Err(error) = &result {
                                context.add("push.dropped", "consume_failed");
                                error!(
                                    consumer = consumer_name,
                                    reason = "consume_failed",
                                    error = ?error,
                                    "push notification dropped"
                                );
                            }

                            result
                        },
                    )
                    .await;
                })
            }
            Ok(None) => Box::pin(ready(())),
            Err(error) => Box::pin(async move {
                error!(
                    consumer = consumer_name,
                    reason = "bad_delivery",
                    error = ?error,
                    "push notification dropped"
                )
            }),
        }
    }
}
