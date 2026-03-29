use amqprs::{BasicProperties, Deliver, channel::{BasicAckArguments, Channel}, consumer::AsyncConsumer};
use async_trait::async_trait;
use revolt_database::{Database, events::rabbit::MessageDeletePayload};
use revolt_search::ElasticsearchClient;

pub struct MessageDeleteConsumer {
    client: ElasticsearchClient,
    database: Database
}

impl MessageDeleteConsumer {
    pub fn new(client: ElasticsearchClient, database: Database) -> Self {
        Self { client, database }
    }
}

#[async_trait]
impl AsyncConsumer for MessageDeleteConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let payload = serde_json::from_slice::<MessageDeletePayload>(&content).expect("Failed to decode message");
        log::debug!("Received message delete {payload:?}");

        if self.client.delete_message(&payload.message_id).await.is_ok() {
            channel.basic_ack(BasicAckArguments::new(deliver.delivery_tag(), false)).await.expect("Failed to ack");
        } else {
            // todo requeue
        }
    }
}