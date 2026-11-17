use std::sync::Arc;

use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use stoat_otel::WideContextRepository;
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct FRAcceptedConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for FRAcceptedConsumer {
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
        let payload: FRAcceptedPayload = serde_json::from_slice(&delivery.data)?;
        context.add("user_id", payload.user.clone());

        debug!("Received FR accept event");

        let sessions = match self.db.fetch_sessions(&payload.user).await {
            Ok(sessions) => sessions,
            Err(error) => {
                context.add("push.dropped", "fetch_sessions_failed");
                warn!(
                    reason = "fetch_sessions_failed",
                    error = ?error,
                    user_id = %payload.user,
                    "push notification dropped"
                );
                return Ok(());
            }
        };

        let mut queued = 0;
        for session in sessions {
            if self
                .publish_push(PayloadKind::FRAccepted(payload.clone()), session)
                .await?
            {
                queued += 1;
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
