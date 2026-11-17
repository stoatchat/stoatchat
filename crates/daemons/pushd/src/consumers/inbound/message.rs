use std::sync::Arc;

use crate::utils::{render_notification_content, Consumer};
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use log::debug;
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct MessageConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for MessageConsumer {
    async fn create(
        db: Database,
        connection: Arc<Connection>,
        channel: Arc<Channel>,
    ) -> Self {
        Self {
            db,
            connection,
            channel,
        }
    }

    fn channel(&self) -> &Arc<Channel> {
        &self.channel
    }

    /// This consumer handles delegating messages into their respective platform queues.
    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let mut payload: MessageSentPayload = serde_json::from_slice(&delivery.data)?;

        if let Ok(body) = render_notification_content(&payload.notification, &self.db).await {
            payload.notification.raw_body = Some(payload.notification.body);
            payload.notification.body = body;
        }

        debug!("Received message event on origin");

        if let Ok(sessions) = self
            .db
            .fetch_sessions_with_subscription(&payload.users)
            .await
        {
            for session in sessions {
                self.publish_push(
                    PayloadKind::MessageNotification(payload.notification.clone()),
                    session,
                )
                .await?;
            }
        }

        Ok(())
    }
}
