use amqprs::{
    BasicProperties, Deliver,
    channel::{BasicAckArguments, BasicRejectArguments, Channel},
    consumer::AsyncConsumer,
};
use async_trait::async_trait;
use revolt_database::{Database, events::rabbit::MessageEditPayload};
use revolt_search::ElasticsearchClient;

pub struct MessageEditConsumer {
    client: ElasticsearchClient,
    database: Database,
}

impl MessageEditConsumer {
    pub fn new(client: ElasticsearchClient, database: Database) -> Self {
        Self { client, database }
    }
}

#[async_trait]
impl AsyncConsumer for MessageEditConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let payload = serde_json::from_slice::<MessageEditPayload>(&content)
            .expect("Failed to decode message");
        log::debug!("Received edit message {payload:?}");

        if self
            .client
            .edit_message(&self.database, payload.message, payload.user)
            .await
            .is_ok()
        {
            channel
                .basic_ack(BasicAckArguments::new(deliver.delivery_tag(), false))
                .await
                .expect("Failed to ack");
        } else {
            channel
                .basic_reject(BasicRejectArguments::new(deliver.delivery_tag(), true))
                .await
                .expect("Failed to reject");
        }
    }
}
