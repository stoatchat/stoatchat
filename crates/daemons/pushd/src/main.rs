#[macro_use]
extern crate log;

use std::sync::Arc;

use lapin::{
    options::{BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable},
    Channel, Connection, ConnectionProperties,
};
use revolt_config::{config, Settings};
use revolt_database::Database;
use tokio::signal::ctrl_c;

mod consumers;
mod utils;
use consumers::{
    inbound::{
        ack::AckConsumer, dm_call::DmCallConsumer, fr_accepted::FRAcceptedConsumer,
        fr_received::FRReceivedConsumer, generic::GenericConsumer,
        mass_mention::MassMessageConsumer, message::MessageConsumer,
    },
    outbound::{apn::ApnsOutboundConsumer, fcm::FcmOutboundConsumer, vapid::VapidOutboundConsumer},
};

use crate::utils::{Consumer, Delegate};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // Configure logging and environment
    revolt_config::configure!(pushd);

    // Setup database
    let db = revolt_database::DatabaseInfo::Auto.connect().await.unwrap();

    let config = config().await;

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

    // An explainer of how this works:
    // The inbound connections are on separate routing keys, such that they only receive the proper payload
    // from their respective api (prod or test).
    // However, the outbound queues that go to the services are routed to receive from both, so that messages
    // sent from beta are still notified on prod, and vice versa.

    // This'll require some interesting shimming if we need to add more events once this is in prod (different payloads between prod and test),
    // but that sounds like a problem for future us.

    channels.push(
        make_queue_and_consume::<GenericConsumer>(
            &db,
            &connection,
            &config,
            &config.pushd.generic_queue,
            &config.pushd.get_generic_routing_key(),
            None,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<MessageConsumer>(
            &db,
            &connection,
            &config,
            &config.pushd.message_queue,
            &config.pushd.get_message_routing_key(),
            None,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<FRReceivedConsumer>(
            &db,
            &connection,
            &config,
            &config.pushd.fr_received_queue,
            &config.pushd.get_fr_received_routing_key(),
            None,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<FRAcceptedConsumer>(
            &db,
            &connection,
            &config,
            &config.pushd.fr_accepted_queue,
            &config.pushd.get_fr_accepted_routing_key(),
            None,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<MassMessageConsumer>(
            &db,
            &connection,
            &config,
            &config.pushd.mass_mention_queue,
            &config.pushd.get_mass_mention_routing_key(),
            None,
        )
        .await,
    );

    channels.push(
        make_queue_and_consume::<DmCallConsumer>(
            &db,
            &connection,
            &config,
            &config.pushd.dm_call_queue,
            &config.pushd.get_dm_call_routing_key(),
            None,
        )
        .await,
    );

    if !config.pushd.apn.pkcs8.is_empty() {
        channels.push(
            make_queue_and_consume::<ApnsOutboundConsumer>(
                &db,
                &connection,
                &config,
                &config.pushd.apn.queue,
                &config.pushd.apn.queue,
                None,
            )
            .await,
        );

        let mut table = FieldTable::default();
        table.insert("x-message-deduplication".into(), AMQPValue::Boolean(true));

        channels.push(
            make_queue_and_consume::<AckConsumer>(
                &db,
                &connection,
                &config,
                &config.pushd.ack_queue,
                &config.pushd.ack_queue,
                Some(table),
            )
            .await,
        );
    }

    if !config.pushd.fcm.auth_uri.is_empty() {
        channels.push(
            make_queue_and_consume::<FcmOutboundConsumer>(
                &db,
                &connection,
                &config,
                &config.pushd.fcm.queue,
                &config.pushd.fcm.queue,
                None,
            )
            .await,
        );
    }

    if !config.pushd.vapid.public_key.is_empty() {
        channels.push(
            make_queue_and_consume::<VapidOutboundConsumer>(
                &db,
                &connection,
                &config,
                &config.pushd.vapid.queue,
                &config.pushd.vapid.queue,
                None,
            )
            .await,
        );
    }

    ctrl_c().await.unwrap();

    for channel in channels {
        let _ = channel.close(0, "close".into()).await;
    }
}

async fn make_queue_and_consume<F>(
    db: &Database,
    connection: &Arc<Connection>,
    config: &Settings,
    queue_name: &str,
    routing_key: &str,
    queue_args: Option<FieldTable>,
) -> Arc<Channel>
where
    F: Consumer,
{
    let channel = Arc::new(connection.create_channel().await.unwrap());

    channel
        .exchange_declare(
            config.pushd.exchange.clone().into(),
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("Failed to declare exchange");

    let mut queue_name = queue_name.to_string();

    if config.pushd.production {
        queue_name += "-prd";
    } else {
        queue_name += "-tst";
    }

    let queue_name = queue_name.as_str();

    let args = QueueDeclareOptions {
        durable: true,
        ..Default::default()
    };

    channel
        .queue_declare(queue_name.into(), args, queue_args.unwrap_or_default())
        .await
        .unwrap();

    channel
        .queue_bind(
            queue_name.into(),
            config.pushd.exchange.clone().into(),
            routing_key.into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect(
            "This probably means the revolt.notifications exchange does not exist in rabbitmq!",
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
        routing_key,
        queue_name,
        consumer.tag()
    );

    let delegate = Delegate(
        F::create(
            db.clone(),
            connection.clone(),
            channel.clone(),
        )
        .await,
    );

    consumer.set_delegate(delegate);

    channel
}
