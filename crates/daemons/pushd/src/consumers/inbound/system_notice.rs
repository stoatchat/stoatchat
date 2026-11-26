use std::sync::Arc;

use crate::utils::{Consumer, record_skip};
use anyhow::Result;
use async_trait::async_trait;
use lapin::{Channel, Connection, message::Delivery};
use revolt_database::{Database, events::rabbit::*};
use stoat_otel::WideContextRepository;

#[derive(Clone)]
#[allow(unused)]
pub struct SystemNoticeConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for SystemNoticeConsumer {
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

    /// This consumer handles delegating messages into their respective platform queues.
    async fn consume(&self, delivery: Delivery, context: &WideContextRepository) -> Result<()> {
        let payload: SystemNoticePayload = serde_json::from_slice(&delivery.data)?;
        context.add("user_id", payload.user.clone());

        debug!("Received system notice event");

        let sessions = match self
            .db
            .fetch_sessions_with_subscription(std::slice::from_ref(&payload.user))
            .await
        {
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
                .publish_push(PayloadKind::SystemNotice(payload.clone()), session)
                .await?
            {
                queued += 1;
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
