use std::sync::Arc;

use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct AckConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for AckConsumer {
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

    /// This consumer processes all acks the platform receives, and sends relevant badge updates to apple platforms.
    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let payload: AckPayload = serde_json::from_slice(&delivery.data)?;

        // Step 1: fetch unreads and don't continue if there's no unreads
        // #[allow(clippy::disallowed_methods)]

        debug!("Processing unreads for {:}", &payload.user_id);

        let unreads = if let Ok(u) = self.db.fetch_unread_mentions(&payload.user_id).await {
            if u.is_empty() {
                debug!(
                    "Discarding unread task (no mentions found) for {:}",
                    &payload.user_id
                );
                return Ok(());
            };

            u
        } else {
            return Ok(());
        };

        if let Ok(sessions) = self.db.fetch_sessions(&payload.user_id).await {
            let config = revolt_config::config().await;
            // Step 2: find any apple sessions, since we don't need to calculate this for anything else.
            // If there's no apple sessions, we can return early
            let mut apple_sessions = sessions
                .into_iter()
                .filter(|session| {
                    if let Some(sub) = &session.subscription {
                        sub.endpoint == "apn"
                    } else {
                        false
                    }
                })
                .peekable();

            if apple_sessions.peek().is_none() {
                debug!(
                    "Discarding unread task (no apn sessions found) for {:}",
                    &payload.user_id
                );
                return Ok(());
            }

            // Step 3: calculate the actual mention count, since we have to send it out
            let mut mention_count = 0;
            for u in &unreads {
                mention_count += u.mentions.as_ref().unwrap().len()
            }

            // Step 4: loop through each apple session and send the badge update
            for session in apple_sessions {
                let service_payload = PayloadToService {
                    notification: PayloadKind::BadgeUpdate(mention_count),
                    user_id: payload.user_id.clone(),
                    session_id: session.id.clone(),
                    token: session.subscription.as_ref().unwrap().auth.clone(),
                    extras: Default::default(),
                };
                let payload = serde_json::to_string(&service_payload)?;

                log::debug!(
                    "Publishing ack to apn session {}",
                    session.subscription.as_ref().unwrap().auth
                );

                self.publish_message(
                    payload.as_bytes(),
                    &config.pushd.exchange,
                    &config.pushd.apn.queue,
                )
                .await?;
            }
        }

        Ok(())
    }
}
