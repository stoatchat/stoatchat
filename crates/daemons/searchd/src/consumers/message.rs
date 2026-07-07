use std::sync::Arc;

use lapin::{Channel, Connection, message::Delivery, options::{BasicAckOptions, BasicRejectOptions}};

use async_trait::async_trait;
use revolt_database::{Database, events::rabbit::MessageCreatePayload, amqp::consumer::Consumer};
use revolt_search::ElasticsearchClient;
use anyhow::Result;

#[allow(unused)]
#[derive(Clone)]
pub struct MessageConsumer {
    client: ElasticsearchClient,
    database: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer<ElasticsearchClient> for MessageConsumer {
    async fn create(database: Database, connection: Arc<Connection>, channel: Arc<Channel>, client: ElasticsearchClient) -> Self {
        Self {
            client,
            database,
            connection,
            channel,
        }
    }

    fn channel(&self) -> &Arc<Channel> {
        &self.channel
    }

    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let payload = serde_json::from_slice::<MessageCreatePayload>(&delivery.data)
            .expect("Failed to decode message");
        log::debug!("Received message {payload:?}");

        if self
            .client
            .index_message(&self.database, payload.message, payload.user)
            .await
            .is_ok()
        {
            self.channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .await
                .expect("Failed to ack");
        } else {
            self.channel
                .basic_reject(delivery.delivery_tag, BasicRejectOptions {
                    requeue: true,
                })
                .await
                .expect("Failed to reject");
        };

        Ok(())
    }
}
