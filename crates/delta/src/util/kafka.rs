use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::ClientConfig;
use rdkafka::producer::future_producer::Delivery;
use rdkafka::error::KafkaError;
use rdkafka::message;
use rdkafka::message::OwnedMessage;

const KAFKA_TOPIC: &'static str = "osprey.actions_input";

/// Wrapper for the Kafka client, managed by Rocket
pub struct KafkaClient(FutureProducer);

impl KafkaClient {
    pub fn connect(host: String) -> Self {
        let producer: FutureProducer = {
            ClientConfig::new()
                .set("bootstrap.servers", host)
                .set("queue.buffering.max.ms", "0") 
                .create()
                .expect("Producer creation failed")
        };
        
        Self(producer)
    }

    pub fn create_record<'a, K: message::ToBytes, P: message::ToBytes>(
        &self
    ) -> FutureRecord<'a, K, P> {
        FutureRecord::to(KAFKA_TOPIC)
    }
    
    pub async fn enqueue<'a, K: message::ToBytes, P: message::ToBytes>(
        &self,
        record: FutureRecord<'a, K, P>
    ) -> Result<Delivery, (KafkaError, OwnedMessage)> {
        self.0.send(record, std::time::Duration::from_secs(1)).await
    }
}
