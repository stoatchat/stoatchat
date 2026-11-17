use std::sync::Arc;

use crate::utils::{record_skip, Consumer};
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use stoat_otel::WideContextRepository;
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct FRReceivedConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for FRReceivedConsumer {
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
        let payload: FRReceivedPayload = serde_json::from_slice(&delivery.data)?;
        context.add("user_id", payload.user.clone());

        debug!("Received FR received event");

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
                record_skip(&payload.user, None, "fetch_sessions_failed");
                return Ok(());
            }
        };

        if sessions.is_empty() {
            record_skip(&payload.user, None, "no_sessions");
        }

        let mut queued = 0;
        for session in sessions {
            if self
                .publish_push(PayloadKind::FRReceived(payload.clone()), session)
                .await?
            {
                queued += 1;
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
