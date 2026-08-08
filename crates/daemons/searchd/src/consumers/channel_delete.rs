use std::sync::Arc;

use lapin::{Channel, Connection, message::Delivery, options::{BasicAckOptions, BasicRejectOptions}};
use async_trait::async_trait;
use revolt_database::{Database, events::rabbit::ChannelDeletePayload, amqp::consumer::Consumer};
use revolt_search::ElasticsearchClient;
use anyhow::Result;

#[allow(unused)]
#[derive(Clone)]
pub struct ChannelDeleteConsumer {
    client: ElasticsearchClient,
    database: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer<ElasticsearchClient> for ChannelDeleteConsumer {
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
        let payload = serde_json::from_slice::<ChannelDeletePayload>(&delivery.data)
            .expect("Failed to decode message");
        log::debug!("Received channel delete {payload:?}");

        if self
            .client
            .delete_channel(&payload.channel_id)
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
