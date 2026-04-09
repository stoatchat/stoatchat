use amqprs::{
    BasicProperties, Deliver,
    channel::{BasicAckArguments, BasicRejectArguments, Channel},
    consumer::AsyncConsumer,
};
use async_trait::async_trait;
use revolt_database::{Database, events::rabbit::ChannelDeletePayload};
use revolt_search::ElasticsearchClient;

#[allow(unused)]
pub struct ChannelDeleteConsumer {
    client: ElasticsearchClient,
    database: Database,
}

impl ChannelDeleteConsumer {
    pub fn new(client: ElasticsearchClient, database: Database) -> Self {
        Self { client, database }
    }
}

#[async_trait]
impl AsyncConsumer for ChannelDeleteConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let payload = serde_json::from_slice::<ChannelDeletePayload>(&content)
            .expect("Failed to decode message");
        log::debug!("Received channel delete {payload:?}");

        if self
            .client
            .delete_channel(&payload.channel_id)
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
