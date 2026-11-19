use futures_lite::stream::StreamExt;
use lapin::{
    ConnectionBuilder, ExchangeKind,
    options::*,
    types::FieldTable,
    uri::{AMQPAuthority, AMQPQueryString, AMQPUri, AMQPUserInfo},
};
use redis_kiss::{AsyncCommands, Conn as RedisConnection, get_connection};
use revolt_config::config;
use revolt_database::{
    amqp_connection_properties, events::rabbit::AckEventPayload, Database, AMQP,
    AMQP_CONNECT_TIMEOUT,
};
use revolt_result::{Result, ToRevoltError};
use serde_json;
use tracing::{debug, error, field::Empty, info, info_span, warn, Instrument, Span};

pub async fn task(db: Database, amqp: AMQP) -> Result<()> {
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

    let connection = tokio::time::timeout(
        AMQP_CONNECT_TIMEOUT,
        ConnectionBuilder::new()
            .expect("Builder")
            .with_uri(uri)
            .with_properties(amqp_connection_properties())
            .connect(),
    )
    .await
    .expect("Timed out connecting to rabbitmq")
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
            let span = info_span!(
                parent: None,
                "crond.ack",
                otel.name = format!("process {}", delivery.routing_key),
                otel.kind = "consumer",
                messaging.system = "rabbitmq",
                messaging.operation.type = "process",
                messaging.destination.name = %delivery.exchange,
                messaging.rabbitmq.destination.routing_key = %delivery.routing_key,
                redelivered = delivery.redelivered,
                user_id = Empty,
                channel_id = Empty,
                otel.status_code = Empty,
            );
            stoat_otel::amqp::set_parent_from_headers(
                &span,
                delivery.properties.headers().as_ref(),
            );

            async {
                let payload = serde_json::from_slice::<AckEventPayload>(&delivery.data);

                if let Ok(payload) = payload {
                    debug!(?payload, "Received ack event");

                    let span = Span::current();
                    span.record("user_id", &payload.user_id);
                    span.record("channel_id", payload.channel_id.as_deref());

                    if let Err(e) = process_channel_ack(
                        &db,
                        &amqp,
                        payload.user_id,
                        payload.channel_id.unwrap(),
                        !delivery.redelivered,
                        &mut redis,
                    )
                    .await
                    {
                        Span::current().record("otel.status_code", "error");
                        error!(error = ?e, "Failed to process ack, requeueing");
                        _ = delivery.reject(BasicRejectOptions { requeue: true }).await;
                    } else {
                        _ = delivery.ack(BasicAckOptions { multiple: false }).await;
                    }
                } else {
                    Span::current().record("otel.status_code", "error");
                    error!(
                        data = %String::from_utf8_lossy(&delivery.data),
                        "Failed to decode ack data"
                    );
                }
            }
            .instrument(span)
            .await;
        }
    }
    Ok(())
}

#[allow(clippy::disallowed_methods)]
async fn process_channel_ack(
    db: &Database,
    amqp: &AMQP,
    user: String,
    channel: String,
    retryable: bool,
    redis: &mut RedisConnection,
) -> Result<()> {
    let key = format!("acker:{user}+{channel}");
    let message_id: Option<String> = redis.get_del(&key).await.to_internal_error()?;

    if let Some(message_id) = &message_id {
        let resp: Result<()> = async {
            let unread = db.fetch_unread(&user, &channel).await?;
            let updated = db.acknowledge_message(&channel, &user, message_id).await?;

            info!(%channel, %user, %message_id, "Set new state for ack");

            if let (Some(before), Some(after)) = (unread, updated) {
                let before_mentions = before.mentions.unwrap_or_default().len();
                let after_mentions = after.mentions.unwrap_or_default().len();

                #[allow(clippy::collapsible_if)]
                if after_mentions < before_mentions {
                    if let Err(err) = amqp
                        .ack_notification_message(
                            user.to_string(),
                            channel.to_string(),
                            message_id.clone(),
                        )
                        .await
                    {
                        error!(
                            reason = "ack_publish_failed",
                            user_id = %user,
                            channel_id = %channel,
                            %message_id,
                            error = ?err,
                            "push notification dropped"
                        );
                    }
                };
            }

            Ok(())
        }
        .await;

        if retryable && let Err(e) = resp {
            let _ = redis.set_ex::<_, _, ()>(&key, &message_id, 30).await;
            Err(e)
        } else {
            resp
        }
    } else {
        warn!(%channel, %user, "No pending ack found");
        Ok(())
    }
}
