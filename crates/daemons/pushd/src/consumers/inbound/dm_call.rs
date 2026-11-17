use std::sync::Arc;

use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use stoat_otel::WideContextRepository;
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct DmCallConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for DmCallConsumer {
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
        let _p: InternalDmCallPayload = serde_json::from_slice(&delivery.data)?;
        let payload = _p.payload;
        context.add("channel_id", payload.channel_id.clone());
        context.add("call.ended", payload.ended);

        debug!("Received dm call start/stop event");

        let (revolt_database::Channel::DirectMessage { recipients, .. }
        | revolt_database::Channel::Group { recipients, .. }) =
            self.db.fetch_channel(&payload.channel_id).await?
        else {
            warn!(
                channel_id = %payload.channel_id,
                "Discarding dm call start/stop event for non-dm/group channel"
            );

            return Ok(());
        };

        let call_recipients = if let Some(user_recipients) = _p.recipients {
            user_recipients
                .into_iter()
                .filter(|user_id| recipients.contains(user_id) && user_id != &payload.initiator_id)
                .collect()
        } else {
            recipients
                .into_iter()
                .filter(|user_id| user_id != &payload.initiator_id)
                .collect::<Vec<_>>()
        };

        context.add("users.count", call_recipients.len() as i64);

        let mut queued = 0;
        for user_id in call_recipients {
            if let Ok(sessions) = self.db.fetch_sessions(&user_id).await {
                for session in sessions {
                    if self
                        .publish_push(PayloadKind::DmCallStartEnd(payload.clone()), session)
                        .await?
                    {
                        queued += 1;
                    }
                }
            }
        }

        context.add("pushes.queued", queued);
        Ok(())
    }
}
