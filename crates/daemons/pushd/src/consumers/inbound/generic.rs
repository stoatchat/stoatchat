use std::sync::Arc;

use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use stoat_otel::WideContextRepository;
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct GenericConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for GenericConsumer {
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
    async fn consume(&self, delivery: Delivery, context: &WideContextRepository) -> Result<()> {
        let payload: MessageSentPayload = serde_json::from_slice(&delivery.data)?;
        context.add("users.count", payload.users.len() as i64);

        debug!("Received message event on origin");

        let mut queued = 0;
        if let Ok(sessions) = self
            .db
            .fetch_sessions_with_subscription(&payload.users)
            .await
        {
            for session in sessions {
                let notification = PayloadKind::MessageNotification(payload.notification.clone());
                if self.publish_push(notification, session).await? {
                    queued += 1;
                }
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
