use std::sync::Arc;

use crate::utils::Consumer;
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

        let mut queued = 0;
        if let Ok(sessions) = self.db.fetch_sessions(&payload.user).await {
            for session in sessions {
                if self
                    .publish_push(PayloadKind::FRReceived(payload.clone()), session)
                    .await?
                {
                    queued += 1;
                }
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
