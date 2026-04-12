use std::net::{Ipv4Addr, SocketAddr};

use axum::{middleware::from_fn_with_state, Router, routing::post};
use revolt_database::{Database, DatabaseInfo};

use axum_macros::FromRef;
use tokio::net::TcpListener;
use serde::Deserialize;
use tracing::{info, warn};

use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::ClientConfig;
use rdkafka::producer::future_producer::Delivery;
use rdkafka::error::KafkaError;
use rdkafka::message;
use rdkafka::message::OwnedMessage;
use rdkafka::message::BorrowedMessage;
use rdkafka::consumer::StreamConsumer;
use rdkafka::consumer::Consumer;
use rdkafka::Message;
 use rdkafka::consumer::CommitMode;



mod api;
mod kafka;

#[derive(FromRef, Clone)]
struct AppState {
    db: Database,
    kafka: kafka::KafkaClient
}

#[derive(Deserialize, Debug)]
struct ExecutionResult {
    __action_id: u64,
    __timestamp: String,
    __error_count: u16, 
    __entity_label_mutations: Vec<String>, 
    ActionName: String,
    UserId: String,
    EventType: String, 
    PostText: String, 
    ContainsHello: bool,
    // Effects
    __ban_user: Option<Vec<String>>
}

const PORT: u16 = 14706;

#[tokio::main]
async fn main() -> Result<(), tokio::io::Error> {
    // Configure logging and environment
    revolt_config::configure!(ospreyd);

    info!("Starting Axum");

    let db = DatabaseInfo::Auto.connect().await.unwrap();
    let kafka = kafka::KafkaClient::connect("127.0.0.1:9092".to_string());

    let state = AppState {
        db,
        kafka
    };

    // Configure Axum and router
    let app = Router::new()
        .route("/ingest", post(api::ingest))
        .with_state(state);

    // Configure TCP listener and bind
    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, PORT));
    let listener = TcpListener::bind(&address).await?;
    let axum_f = axum::serve(listener, app.into_make_service());

    let kafka_f = tokio::spawn(async move {
        let consumer: StreamConsumer = {
            ClientConfig::new()
                .set("bootstrap.servers", "127.0.0.1:9092")
                .set("group.id", "osprey")
                .set("enable.partition.eof", "false")
                .set("session.timeout.ms", "6000")
                .set("enable.auto.commit", "true")
                .create()
                .expect("Consumer creation failed")
        };
        
        consumer
            .subscribe(&["osprey.execution_results"])
            .expect("Can't subscribe to specified topics");

        info!("Subscribed");

        loop {
            match consumer.recv().await {
                Err(e) => warn!("Kafka error: {}", e),
                Ok(m) => {
                    let payload = match m.payload_view::<str>() {
                        None => "",
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            warn!("Error while deserializing message payload: {:?}", e);
                            ""
                        }
                    };
                    let payload = serde_json::from_str::<ExecutionResult>(&payload);
                    dbg!(payload);
                    info!(
                        "key: '{:?}', , topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                        m.key(),
                        m.topic(),
                        m.partition(),
                        m.offset(),
                        m.timestamp()
                    );

                    consumer.commit_message(&m, CommitMode::Async).unwrap();
                }
            };
        }
    });

    tokio::join!(
        axum_f,
        kafka_f
    );

    Ok(())
}
