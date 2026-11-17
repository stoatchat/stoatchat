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

#[async_trait]
pub trait Consumer: Clone + Send + Sync + 'static {
    async fn create(
        db: Database,
        connection: Arc<Connection>,
        channel: Arc<Channel>,
    ) -> Self;
    fn channel(&self) -> &Arc<Channel>;
    async fn consume(&self, delivery: Delivery) -> Result<()>;

    async fn publish_message_with_options(
        &self,
        payload: &[u8],
        exchange: &str,
        routing_key: &str,
        options: BasicPublishOptions,
        properties: BasicProperties,
    ) -> Result<(), AMQPError> {
        let channel = self.channel();

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

    /// Queue a push for the session on its provider's outbound queue,
    /// sessions without a subscription are skipped
    async fn publish_push(&self, notification: PayloadKind, session: Session) -> Result<()> {
        let Some(sub) = session.subscription else {
            return Ok(());
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

        Ok(())
    }
}

pub struct Delegate<C: Consumer>(pub C);

impl<C: Consumer> ConsumerDelegate for Delegate<C> {
    fn on_new_delivery(
        &self,
        delivery: DeliveryResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        match delivery {
            Ok(Some(delivery)) => {
                let consumer = self.0.clone();

                Box::pin(async move {
                    if let Err(error) = consumer.consume(delivery).await {
                        error!(error = ?error, "Failed to process delivery");
                    };
                })
            }
            Ok(None) => Box::pin(ready(())),
            Err(error) => {
                Box::pin(async move { error!(error = ?error, "Received bad delivery") })
            }
        }
    }
}
