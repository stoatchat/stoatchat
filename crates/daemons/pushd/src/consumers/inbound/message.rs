use std::sync::Arc;

use crate::utils::{
    Consumer, record_skip, record_skips, record_users_without_sessions, render_notification_content,
};
use anyhow::Result;
use async_trait::async_trait;
use lapin::{Channel, Connection, message::Delivery};
use revolt_database::{Database, Message, SystemMessage, events::rabbit::*};
use revolt_models::v0::{MessageAuthor, PushNotification};
use stoat_otel::WideContextRepository;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Author ID used for system messages
const SYSTEM_AUTHOR_ID: &str = "00000000000000000000000000";

fn system_actor(message: &Message) -> Option<&str> {
    match message.system.as_ref()? {
        SystemMessage::UserAdded { by, .. }
        | SystemMessage::UserRemove { by, .. }
        | SystemMessage::ChannelRenamed { by, .. }
        | SystemMessage::ChannelDescriptionChanged { by }
        | SystemMessage::ChannelIconChanged { by }
        | SystemMessage::MessagePinned { by, .. }
        | SystemMessage::MessageUnpinned { by, .. }
        | SystemMessage::CallStarted { by, .. } => Some(by),
        SystemMessage::UserJoined { id }
        | SystemMessage::UserLeft { id }
        | SystemMessage::UserKicked { id }
        | SystemMessage::UserBanned { id } => Some(id),
        SystemMessage::ChannelOwnershipChanged { from, .. } => Some(from),
        SystemMessage::Text { .. } => None,
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct MessageConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

impl MessageConsumer {
    /// Build the push notification, resolving the author for the name and icon
    async fn build_notification(
        &self,
        message: &Message,
        channel: &revolt_database::Channel,
    ) -> PushNotification {
        // system messages don't keep the acting user, so they fall back to the generic author
        let user = if message.webhook.is_none() && message.author != SYSTEM_AUTHOR_ID {
            match self.db.fetch_user(&message.author).await {
                Ok(user) => Some(user.into_known_static(false).await),
                Err(error) => {
                    debug!(?error, "Failed to fetch message author");
                    None
                }
            }
        } else {
            None
        };

        let webhook = if message.webhook.is_some() {
            match self.db.fetch_webhook(&message.author).await {
                Ok(webhook) => Some(webhook.into()),
                Err(error) => {
                    debug!(?error, "Failed to fetch message webhook");
                    None
                }
            }
        } else {
            None
        };

        let author = user
            .as_ref()
            .map(MessageAuthor::User)
            .or(webhook.as_ref().map(MessageAuthor::Webhook));

        PushNotification::from(
            message.clone().into_model(user.clone(), None),
            author,
            channel.clone().into(),
        )
        .await
    }

    /// Hand the message off to the mass mention consumer
    async fn publish_mass_mention(
        &self,
        server_id: String,
        notification: PushNotification,
    ) -> Result<()> {
        let config = revolt_config::config().await;

        let payload = MassMessageSentPayload {
            notifications: vec![notification],
            server_id,
        };
        let payload = serde_json::to_string(&payload)?;

        self.publish_message(
            payload.as_bytes(),
            &config.pushd.exchange,
            &config.pushd.get_mass_mention_routing_key(),
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Consumer for MessageConsumer {
    async fn create(db: Database, connection: Arc<Connection>, channel: Arc<Channel>) -> Self {
        Self {
            db,
            connection,
            channel,
        }
    }

    fn channel(&self) -> &Arc<Channel> {
        &self.channel
    }

    /// This consumer handles adding mentions to unreads and delegating messages into their respective platform queues.
    async fn consume(&self, delivery: Delivery, context: &WideContextRepository) -> Result<()> {
        let payload: MessageSentPayload = serde_json::from_slice(&delivery.data)?;
        let message = payload.message;

        context.add("message_id", message.id.clone());
        context.add("channel_id", message.channel.clone());

        // on the span too, so push.skipped events can be tied back to the message
        let span = tracing::Span::current();
        span.set_attribute("message_id", message.id.clone());
        span.set_attribute("channel_id", message.channel.clone());

        debug!("Received message event on origin");

        if message.has_suppressed_notifications() {
            context.add("push.dropped", "suppressed");
            return Ok(());
        }

        // channel might've been deleted in the meantime
        let channel = match self.db.fetch_channel(&message.channel).await {
            Ok(channel) => channel,
            Err(error) => {
                context.add("push.dropped", "fetch_channel_failed");
                warn!(
                    reason = "fetch_channel_failed",
                    error = ?error,
                    "push notification dropped"
                );
                return Ok(());
            }
        };

        let is_dm_or_group = matches!(
            channel,
            revolt_database::Channel::DirectMessage { .. } | revolt_database::Channel::Group { .. }
        );
        let is_mass_mention = message.contains_mass_push_mention();

        if !(is_dm_or_group || message.mentions.is_some() || is_mass_mention) {
            return Ok(());
        }

        // nobody should be notified about their own message
        let sender = system_actor(&message).unwrap_or(&message.author);

        let recipients: Vec<String> = match &channel {
            revolt_database::Channel::DirectMessage { recipients, .. }
            | revolt_database::Channel::Group { recipients, .. } => recipients.clone(),
            revolt_database::Channel::TextChannel { .. } => {
                message.mentions.clone().unwrap_or_default()
            }
            _ => vec![],
        }
        .into_iter()
        .filter(|uid| uid != sender)
        .collect();

        context.add("recipients.count", recipients.len() as i64);

        if !recipients.is_empty()
            && let Err(err) = self
                .db
                .add_mention_to_many_unreads(
                    &message.channel,
                    &recipients,
                    std::slice::from_ref(&message.id),
                )
                .await
        {
            revolt_config::capture_error(&err);
            error!(error = ?err, "Failed to add mentions to unreads");
        }

        let mut notification = self.build_notification(&message, &channel).await;

        if is_mass_mention {
            if let revolt_database::Channel::TextChannel { server, .. } = &channel {
                // sent before rendering, the mass mention consumer renders it itself
                if let Err(error) = self
                    .publish_mass_mention(server.clone(), notification.clone())
                    .await
                {
                    error!(?error, "Failed to send mass mention event");
                }
            } else {
                info!(
                    "Non-server channel {} when sending mass mention event",
                    channel.id()
                );
            }
        }

        if recipients.is_empty() {
            context.add("pushes.queued", 0);
            return Ok(());
        }

        let online = revolt_presence::filter_online(&recipients).await;
        let users: Vec<String> = if stoat_otel::extra_tracing() {
            recipients
                .into_iter()
                .filter(|id| {
                    let is_online = online.contains(id);
                    if is_online {
                        record_skip(id, None, "online");
                    }
                    !is_online
                })
                .collect()
        } else {
            online.into_iter().collect()
        };

        context.add("users.count", users.len() as i64);

        if users.is_empty() {
            context.add("pushes.queued", 0);
            return Ok(());
        }

        match render_notification_content(&notification, &self.db).await {
            Ok(body) => {
                notification.raw_body = Some(notification.body);
                notification.body = body;
            }
            Err(error) => debug!(?error, "Failed to render notification, sending raw body"),
        }

        let sessions = match self.db.fetch_sessions_with_subscription(&users).await {
            Ok(sessions) => sessions,
            Err(error) => {
                context.add("push.dropped", "fetch_sessions_failed");
                warn!(
                    reason = "fetch_sessions_failed",
                    error = ?error,
                    users = ?users,
                    "push notification dropped"
                );
                record_skips(&users, "fetch_sessions_failed");
                return Ok(());
            }
        };

        record_users_without_sessions(&users, &sessions);

        let mut queued = 0;
        for session in sessions {
            let notification = PayloadKind::MessageNotification(notification.clone());
            if self.publish_push(notification, session).await? {
                queued += 1;
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
