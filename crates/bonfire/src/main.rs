use std::env;

use amqprs::{
    channel::{
        BasicConsumeArguments, Channel, ExchangeDeclareArguments, QueueBindArguments,
        QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use async_std::net::TcpListener;
use async_trait::async_trait;
use redis_kiss::AsyncCommands;
use revolt_database::util::rabbit::set_rabbitmq_connection;
use revolt_presence::clear_region;

#[macro_use]
extern crate log;

pub mod config;
pub mod events;

mod database;
mod websocket;

#[async_std::main]
async fn main() {
    // Configure requirements for Bonfire.
    revolt_config::configure!(events);
    database::connect().await;

    // Clean up the current region information.
    let no_clear_region = env::var("NO_CLEAR_PRESENCE").unwrap_or_else(|_| "0".into()) == "1";
    if !no_clear_region {
        clear_region(None).await;
    }

    // Setup a TCP listener to accept WebSocket connections on.
    // By default, we bind to port 14703 on all interfaces.
    let bind = env::var("HOST").unwrap_or_else(|_| "0.0.0.0:14703".into());
    info!("Listening on host {bind}");
    let try_socket = TcpListener::bind(bind).await;
    let listener = try_socket.expect("Failed to bind");

    let config = revolt_config::config().await;

    let rmq_conn = Connection::open(&OpenConnectionArguments::new(
        &config.rabbit.host,
        config.rabbit.port,
        &config.rabbit.username,
        &config.rabbit.password,
    ))
    .await
    .expect("Failed to connect to RabbitMQ");

    set_rabbitmq_connection(rmq_conn.clone());

    if std::env::var("ENABLE_RABBITMQ_INGRESS").as_deref().is_ok_and(|v| v == "1") {
        let channel = rmq_conn
            .open_channel(None)
            .await
            .expect("Failed to open RabbitMQ channel.");

        channel
            .exchange_declare(
                ExchangeDeclareArguments::new("events", "fanout")
                    .durable(true)
                    .finish(),
            )
            .await
            .expect("Failed to declare exchange");

        channel
            .queue_declare(QueueDeclareArguments::new("events").durable(true).finish())
            .await
            .expect("Failed to declare queue");

        channel
            .queue_bind(QueueBindArguments::new("events", "events", "events"))
            .await
            .expect("Failed to bind queue");

        channel
            .basic_consume(
                RabbitToRedisConsumer,
                BasicConsumeArguments::new("events", "")
                    .manual_ack(false)
                    .finish(),
            )
            .await
            .expect("Failed to consume channel");
    }

    // Start accepting new connections and spawn a client for each connection.
    while let Ok((stream, addr)) = listener.accept().await {
        async_std::task::spawn(async move {
            info!("User connected from {addr:?}");
            websocket::client(database::get_db(), stream, addr).await;
            info!("User disconnected from {addr:?}");
        });
    }
}

struct RabbitToRedisConsumer;

#[async_trait]
impl AsyncConsumer for RabbitToRedisConsumer {
    async fn consume(
        &mut self,
        _channel: &Channel,
        _deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let mut redis_conn = redis_kiss::get_connection()
            .await
            .expect("Failed to connect to Redis.");

        let pubsub_channel = basic_properties
            .headers()
            .expect("No headers")
            .get(&"c".try_into().unwrap())
            .expect("No channel header")
            .to_string();

        redis_conn
            .publish(pubsub_channel, content)
            .await
            .expect("failed to publish")
    }
}
