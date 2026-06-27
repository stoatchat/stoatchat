use std::{
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
use log::debug;
use revolt_database::Database;

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
        debug!("Sent message to queue for target {}", routing_key);

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
                    if let Err(e) = consumer.consume(delivery).await {
                        revolt_config::capture_anyhow(&e);
                        log::error!("{e:?}");
                    };
                })
            }
            Ok(None) => Box::pin(ready(())),
            Err(e) => Box::pin(async move { log::error!("Received bad delivery: {e:?}") }),
        }
    }
}
