use amqprs::{
    channel::{
        BasicConsumeArguments, Channel, ExchangeDeclareArguments, QueueBindArguments,
        QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::AsyncConsumer,
};
use revolt_config::{Settings, config, configure};
use revolt_database::DatabaseInfo;
use revolt_search::ElasticsearchClient;
use tokio::{signal::ctrl_c, spawn};

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

    if std::env::var("REMAKE_MESSAGES_INDEX").as_deref().is_ok_and(|v| v == "1") {
        if client.delete_indexes().await.is_err() {
            log::info!("Index does not existing, skipping.")
        };

        client.setup_indexes().await.unwrap();
    };

    let connection = Connection::open(&OpenConnectionArguments::new(
        &config.rabbit.host,
        config.rabbit.port,
        &config.rabbit.username,
        &config.rabbit.password,
    ))
    .await
    .expect("Failed to connect to RabbitMQ");

    let mut channels = Vec::new();

    channels.push(
        make_queue_and_consume(
            &config,
            &connection,
            &config.elasticsearch.message_queue,
            consumers::MessageConsumer::new(client.clone(), db.clone()),
        )
        .await,
    );

    channels.push(
        make_queue_and_consume(
            &config,
            &connection,
            &config.elasticsearch.message_edit_queue,
            consumers::MessageEditConsumer::new(client.clone(), db.clone()),
        )
        .await,
    );

    channels.push(
        make_queue_and_consume(
            &config,
            &connection,
            &config.elasticsearch.message_delete_queue,
            consumers::MessageDeleteConsumer::new(client.clone(), db.clone()),
        )
        .await,
    );

    channels.push(
        make_queue_and_consume(
            &config,
            &connection,
            &config.elasticsearch.channel_delete_queue,
            consumers::ChannelDeleteConsumer::new(client.clone(), db.clone()),
        )
        .await,
    );

    let mut task = None;

    if std::env::var("INDEX_ALL_MESSAGES").as_deref().is_ok_and(|v| v == "1") {
        task = Some(spawn(index::index_existing_messages(db, client.clone())));
    }

    ctrl_c().await.expect("Failed to wait for ctrl-c");

    for channel in channels {
        channel.close().await.unwrap();
    }

    if let Some(task) = task {
        task.abort();
    }
}

async fn make_queue_and_consume<F: AsyncConsumer + Send + 'static>(
    config: &Settings,
    connection: &Connection,
    queue: &str,
    consumer: F,
) -> Channel {
    let channel = connection.open_channel(None).await.unwrap();

    channel
        .exchange_declare(
            ExchangeDeclareArguments::new(&config.elasticsearch.exchange, "direct")
                .durable(true)
                .finish(),
        )
        .await
        .expect("Failed to declare pushd exchange");

    _ = channel
        .queue_declare(QueueDeclareArguments::new(queue).durable(true).finish())
        .await
        .unwrap()
        .unwrap();

    channel
        .queue_bind(QueueBindArguments::new(
            queue,
            &config.elasticsearch.exchange,
            queue,
        ))
        .await
        .expect("This probably means the revolt.messages exchange does not exist in rabbitmq!");

    let args = BasicConsumeArguments::new(queue, "")
        .manual_ack(true)
        .finish();

    let tag = channel.basic_consume(consumer, args).await.unwrap();
    log::info!("Consuming queue {queue}, tag {tag}");

    channel
}

#[tokio::main]
async fn main() {
    _main().await
}
