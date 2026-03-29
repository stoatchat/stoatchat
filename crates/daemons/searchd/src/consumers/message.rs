use amqprs::{BasicProperties, Deliver, channel::{BasicAckArguments, Channel}, consumer::AsyncConsumer};
use async_trait::async_trait;
use revolt_database::{Database, Message};
use revolt_search::ElasticsearchClient;

pub struct MessageConsumer {
    client: ElasticsearchClient,
    database: Database
}

impl MessageConsumer {
    pub fn new(client: ElasticsearchClient, database: Database) -> Self {
        Self { client, database }
    }
}

#[async_trait]
impl AsyncConsumer for MessageConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let message = serde_json::from_slice::<Message>(&content).expect("Failed to decode message");
        log::debug!("Received message {message:?}");

        if self.client.index_message(&self.database, message).await.is_ok() {
            channel.basic_ack(BasicAckArguments::new(deliver.delivery_tag(), false)).await.expect("Failed to ack");
        } else {
            // todo requeue
        }
    }
}