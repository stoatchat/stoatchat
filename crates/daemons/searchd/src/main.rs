use lapin::{
    options::{BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable},
    Channel, Connection, ConnectionProperties,
};
use log::info;
use revolt_config::{Settings, config, configure};
use revolt_database::{Database, DatabaseInfo, amqp::consumer::{Consumer, Delegate}};
use revolt_search::ElasticsearchClient;
use tokio::{signal::ctrl_c, spawn};
use std::{marker::PhantomData, sync::Arc};

mod consumers;
mod index;

async fn _main() {
    configure!(api);
    let config = config().await;

    let db = DatabaseInfo::Auto.connect().await.unwrap();

    let client = ElasticsearchClient::new(
        &config.elasticsearch.host,
        config.elasticsearch.port,
        config.elasticsearch.api_key.clone(),
    );

    if std::env::var("REMAKE_MESSAGES_INDEX")
        .as_deref()
        .is_ok_and(|v| v == "1")
    {
        if client.delete_indexes().await.is_err() {
            log::info!("Index does not existing, skipping.")
        };

        client.setup_indexes().await.unwrap();
    };

    let connection = Arc::new(
        Connection::connect(
            &format!(
                "amqp://{}:{}@{}:{}",
                &config.rabbit.username,
                &config.rabbit.password,
                &config.rabbit.host,
                &config.rabbit.port,
            ),
            ConnectionProperties::default(),
        )
        .await
        .expect("Failed to connect to RabbitMQ"),
    );

    let mut channels = Vec::new();

    channels.push(
        make_queue_and_consume::<consumers::MessageConsumer>(
            &client,
            &db,
            &connection,
            &config,
            &config.elasticsearch.message_queue,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<consumers::MessageEditConsumer>(
            &client,
            &db,
            &connection,
            &config,
            &config.elasticsearch.message_edit_queue,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<consumers::MessageDeleteConsumer>(
            &client,
            &db,
            &connection,
            &config,
            &config.elasticsearch.message_delete_queue,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<consumers::ChannelDeleteConsumer>(
            &client,
            &db,
            &connection,
            &config,
            &config.elasticsearch.channel_delete_queue,
        )
        .await,
    );

    let mut task = None;

    if std::env::var("INDEX_ALL_MESSAGES")
        .as_deref()
        .is_ok_and(|v| v == "1")
    {
        task = Some(spawn(index::index_existing_messages(db, client.clone())));
    }

    ctrl_c().await.unwrap();

    for channel in channels {
        let _ = channel.close(0, "close".into()).await;
    }

    if let Some(task) = task {
        task.abort();
    }
}

async fn make_queue_and_consume<F: Consumer<ElasticsearchClient>>(
    client: &ElasticsearchClient,
    db: &Database,
    connection: &Arc<Connection>,
    config: &Settings,
    queue_name: &str,
) -> Arc<Channel> {
    let channel = Arc::new(connection.create_channel().await.unwrap());

    channel
        .exchange_declare(
            config.elasticsearch.exchange.clone().into(),
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("Failed to declare exchange");

    let mut table = FieldTable::default();
    table.insert("x-queue-type".try_into().unwrap(), AMQPValue::LongString("quorum".into()));

    let args = QueueDeclareOptions {
        durable: true,
        ..Default::default()
    };

    channel
        .queue_declare(queue_name.into(), args, table)
        .await
        .unwrap();

    channel
        .queue_bind(
            queue_name.into(),
            config.elasticsearch.exchange.clone().into(),
            queue_name.into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect(
            "This probably means the revolt.messages exchange does not exist in rabbitmq!",
        );


    let consumer = channel
        .basic_consume(
            queue_name.into(),
            "".into(),
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();
    info!(
        "Consuming routing key {} as queue {}, tag {}",
        queue_name,
        queue_name,
        consumer.tag()
    );

    let delegate = Delegate::new(
        F::create(
            db.clone(),
            connection.clone(),
            channel.clone(),
            client.clone(),
        )
        .await,
    );

    consumer.set_delegate(delegate);

    channel
}

#[tokio::main]
async fn main() {
    _main().await
}
