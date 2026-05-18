use futures_lite::stream::StreamExt;
use lapin::{
    options::*,
    types::FieldTable,
    uri::{AMQPAuthority, AMQPQueryString, AMQPUri, AMQPUserInfo},
    ConnectionBuilder, ConnectionProperties, ExchangeKind,
};
use log::info;
use redis_kiss::{get_connection, AsyncCommands, Conn as RedisConnection};
use revolt_config::config;
use revolt_database::{events::rabbit::AckEventPayload, Database};
use revolt_result::{Result, ToRevoltError};
use serde_json;

pub async fn task(db: Database) -> Result<()> {
    let config = config().await;

    let mut redis = get_connection()
        .await
        .expect("Failed to get redis connection");

    let uri = AMQPUri {
        scheme: lapin::uri::AMQPScheme::AMQP,
        authority: AMQPAuthority {
            userinfo: AMQPUserInfo {
                username: config.rabbit.username,
                password: config.rabbit.password,
            },
            host: config.rabbit.host,
            port: config.rabbit.port,
        },
        vhost: "/".to_string(),
        query: AMQPQueryString::default(),
    };

    let connection = ConnectionBuilder::new()
        .expect("Builder")
        .with_uri(uri)
        .with_properties(ConnectionProperties::default())
        .connect()
        .await
        .expect("Failed to connect to rabbitmq");

    let reader_channel = connection
        .create_channel()
        .await
        .expect("Failed to create channel");

    reader_channel
        .exchange_declare(
            config.rabbit.default_exchange.clone().into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("Failed to declare exchange");

    reader_channel
        .queue_declare(
            config.rabbit.queues.acks.clone().into(),
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("Failed to bind queue");

    reader_channel
        .queue_bind(
            config.rabbit.queues.acks.clone().into(),
            config.rabbit.default_exchange.into(),
            config.rabbit.queues.acks.clone().into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Failed to bind channel");

    let mut consumer = reader_channel
        .basic_consume(
            config.rabbit.queues.acks.into(),
            "crond-ack-consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Failed to create consumer");

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let payload: std::result::Result<AckEventPayload, _> =
                serde_json::from_slice(&delivery.data);
            if let Ok(payload) = payload {
                info!("{:?}", payload);
                if let Err(e) = process_channel_ack(
                    &db,
                    payload.user_id,
                    payload.channel_id.unwrap(),
                    &mut redis,
                )
                .await
                {
                    revolt_config::capture_error(&e);
                    _ = delivery.reject(BasicRejectOptions { requeue: false }).await;
                } else {
                    _ = delivery.ack(BasicAckOptions { multiple: false }).await;
                }
            } else {
                revolt_config::capture_message(
                    format!("Failed to decode ack data: {:?}", delivery.data).as_str(),
                    revolt_config::Level::Error,
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::disallowed_methods)]
async fn process_channel_ack(
    db: &Database,
    user: String,
    channel: String,
    redis: &mut RedisConnection,
) -> Result<()> {
    let message_id: Option<String> = redis
        .get_del(format!("acker:{user}+{channel}"))
        .await
        .to_internal_error()?;

    if let Some(message_id) = message_id {
        // This will be uncommented eventually, but we need to sort out the transition to lapin first. For now we'll simply disable the badge update logic.
        // We also drop a db request as a bonus.

        //let unread = db.fetch_unread(&user, &channel).await?;
        let _updated = db.acknowledge_message(&channel, &user, &message_id).await?;
        info!("Set new state for ack: {}:{}:{}", channel, user, message_id);

        // if let (Some(before), Some(after)) = (unread, updated) {
        //     let before_mentions = before.mentions.unwrap_or_default().len();
        //     let after_mentions = after.mentions.unwrap_or_default().len();

        //     let mentions_acked = before_mentions - after_mentions;

        //     if mentions_acked > 0 {
        //         if let Err(err) = amqp
        //             .ack_message(user.to_string(), channel.to_string(), payload.message_id)
        //             .await
        //         {
        //             revolt_config::capture_error(&err);
        //         }
        //     };
        // }

        Ok(())
    } else {
        Err(message_id.to_internal_error().expect_err("no err"))
    }
}
