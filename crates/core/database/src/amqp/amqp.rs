use std::collections::HashSet;
use std::sync::Arc;

use crate::events::rabbit::*;
use crate::User;
use lapin::{
    options::BasicPublishOptions,
    protocol::basic::AMQPProperties,
    types::{AMQPValue, FieldTable},
    Channel, Connection, ConnectionProperties, Error as AMQPError,
};
use revolt_models::v0::PushNotification;
use revolt_presence::filter_online;
use revolt_result::Result;

use serde_json::to_string;

#[derive(Clone)]
pub struct AMQP {
    friend_request_accepted: Arc<Channel>,
    friend_request_received: Arc<Channel>,
    generic_message: Arc<Channel>,
    message_sent: Arc<Channel>,
    mass_mention_message_sent: Arc<Channel>,
    ack_notification_message: Arc<Channel>,
    dm_call_updated: Arc<Channel>,
    process_ack: Arc<Channel>,
    #[allow(unused)]
    connection: Arc<Connection>,
}

impl AMQP {
    pub async fn new(connection: Arc<Connection>) -> Self {
        Self {
            friend_request_accepted: Self::create_channel(&connection).await,
            friend_request_received: Self::create_channel(&connection).await,
            generic_message: Self::create_channel(&connection).await,
            message_sent: Self::create_channel(&connection).await,
            mass_mention_message_sent: Self::create_channel(&connection).await,
            ack_notification_message: Self::create_channel(&connection).await,
            dm_call_updated: Self::create_channel(&connection).await,
            process_ack: Self::create_channel(&connection).await,
            connection,
        }
    }

    pub async fn new_auto() -> Self {
        let config = revolt_config::config().await;

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

        Self::new(connection).await
    }

    async fn create_channel(connection: &Connection) -> Arc<Channel> {
        Arc::new(
            connection
                .create_channel()
                .await
                .expect("Failed to create channel"),
        )
    }

    pub async fn friend_request_accepted(
        &self,
        accepted_request_user: &User,
        sent_request_user: &User,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;
        let payload = FRAcceptedPayload {
            accepted_user: accepted_request_user.to_owned(),
            user: sent_request_user.id.clone(),
        };
        let payload = to_string(&payload).unwrap();

        debug!(
            "Sending friend request accept payload on channel {}: {}",
            config.pushd.get_fr_accepted_routing_key(),
            payload
        );

        self.friend_request_accepted
            .basic_publish(
                config.pushd.exchange.clone().into(),
                config.pushd.get_fr_accepted_routing_key().into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    pub async fn friend_request_received(
        &self,
        received_request_user: &User,
        sent_request_user: &User,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;
        let payload = FRReceivedPayload {
            from_user: sent_request_user.to_owned(),
            user: received_request_user.id.clone(),
        };
        let payload = to_string(&payload).unwrap();

        debug!(
            "Sending friend request received payload on channel {}: {}",
            config.pushd.get_fr_received_routing_key(),
            payload
        );

        self.friend_request_received
            .basic_publish(
                config.pushd.exchange.clone().into(),
                config.pushd.get_fr_received_routing_key().into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    pub async fn generic_message(
        &self,
        user: &User,
        title: String,
        body: String,
        icon: Option<String>,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;
        let payload = GenericPayload {
            title,
            body,
            icon,
            user: user.to_owned(),
        };
        let payload = to_string(&payload).unwrap();

        debug!(
            "Sending generic payload on channel {}: {}",
            config.pushd.get_generic_routing_key(),
            payload
        );

        self.generic_message
            .basic_publish(
                config.pushd.exchange.clone().into(),
                config.pushd.get_generic_routing_key().into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    pub async fn message_sent(
        &self,
        recipients: Vec<String>,
        payload: PushNotification,
    ) -> Result<(), AMQPError> {
        if recipients.is_empty() {
            return Ok(());
        }

        let config = revolt_config::config().await;

        let online_ids = filter_online(&recipients).await;
        let recipients = (&recipients.into_iter().collect::<HashSet<String>>() - &online_ids)
            .into_iter()
            .collect::<Vec<String>>();

        let payload = MessageSentPayload {
            notification: payload,
            users: recipients,
        };
        let payload = to_string(&payload).unwrap();

        debug!(
            "Sending message payload on channel {}: {}",
            config.pushd.get_message_routing_key(),
            payload
        );

        self.message_sent
            .basic_publish(
                config.pushd.exchange.clone().into(),
                config.pushd.get_message_routing_key().into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    pub async fn mass_mention_message_sent(
        &self,
        server_id: String,
        payload: Vec<PushNotification>,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;

        let payload = MassMessageSentPayload {
            notifications: payload,
            server_id,
        };
        let payload = to_string(&payload).unwrap();

        let routing_key = config.pushd.get_mass_mention_routing_key();

        debug!(
            "Sending mass mention payload on channel {}: {}",
            routing_key, payload
        );

        self.mass_mention_message_sent
            .basic_publish(
                config.pushd.exchange.clone().into(),
                routing_key.into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    /// # Sends an ack to pushd to update badges on iPhones.
    /// Not to be confused with the process_ack function, which handles sending all acks to crond for processing.
    pub async fn ack_notification_message(
        &self,
        user_id: String,
        channel_id: String,
        message_id: String,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;

        let payload = AckPayload {
            user_id: user_id.clone(),
            channel_id: channel_id.clone(),
            message_id,
        };
        let payload = to_string(&payload).unwrap();

        info!(
            "Sending ack payload on channel {}: {}",
            config.pushd.ack_queue, payload
        );

        let mut headers = FieldTable::default();
        headers.insert(
            "x-deduplication-header".into(),
            AMQPValue::LongString(format!("{}-{}", &user_id, &channel_id).into()),
        );

        self.ack_notification_message
            .basic_publish(
                config.pushd.exchange.clone().into(),
                config.pushd.ack_queue.into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    /// # DM Call Update
    /// Used to send an update about a DM call, eg. start or end of a call.
    /// Recipients can be used to narrow the scope of recipients, otherwise all recipients will be notified.
    /// `ended` refers to the ringing period, not necessarily the call itself.
    pub async fn dm_call_updated(
        &self,
        initiator_id: &str,
        channel_id: &str,
        started_at: Option<&str>,
        ended: bool,
        recipients: Option<Vec<String>>,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;

        let payload = InternalDmCallPayload {
            payload: DmCallPayload {
                initiator_id: initiator_id.to_string(),
                channel_id: channel_id.to_string(),
                started_at: started_at.map(|f| f.to_string()),
                ended,
            },
            recipients,
        };
        let payload = to_string(&payload).unwrap();

        debug!(
            "Sending dm call update payload on channel {}: {}",
            config.pushd.get_dm_call_routing_key(),
            payload
        );

        self.dm_call_updated
            .basic_publish(
                config.pushd.exchange.clone().into(),
                config.pushd.get_dm_call_routing_key().into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }

    /// # Send an ack to crond for processing
    pub async fn process_ack(
        &self,
        user_id: &str,
        channel_id: Option<&str>,
        server_id: Option<&str>,
    ) -> Result<(), AMQPError> {
        let config = revolt_config::config().await;

        let payload = AckEventPayload {
            user_id: user_id.to_string(),
            channel_id: channel_id.map(|value| value.to_string()),
            server_id: server_id.map(|value| value.to_string()),
        };
        let payload = to_string(&payload).unwrap();

        info!(
            "Sending ack processor event on exchange {}, channel {}: {}",
            config.rabbit.default_exchange, config.rabbit.queues.acks, payload
        );

        self.process_ack
            .basic_publish(
                config.rabbit.default_exchange.clone().into(),
                config.rabbit.queues.acks.into(),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                AMQPProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await?;

        Ok(())
    }
}
