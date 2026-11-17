use std::sync::Arc;

use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
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
    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let payload: FRAcceptedPayload = serde_json::from_slice(&delivery.data)?;

        debug!("Received FR accept event");

        if let Ok(sessions) = self.db.fetch_sessions(&payload.user).await {
            for session in sessions {
                self.publish_push(PayloadKind::FRAccepted(payload.clone()), session)
                    .await?;
            }
        }

        Ok(())
    }
}
