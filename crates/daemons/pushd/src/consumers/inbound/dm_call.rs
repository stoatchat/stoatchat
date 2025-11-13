use std::collections::HashMap;

use crate::consumers::inbound::internal::*;
use amqprs::{
    channel::{BasicPublishArguments, Channel},
    connection::Connection,
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use revolt_database::{events::rabbit::*, Database};

pub struct DmCallConsumer {
    #[allow(dead_code)]
    db: Database,
    authifier_db: authifier::Database,
    conn: Option<Connection>,
    channel: Option<Channel>,
}

impl Channeled for DmCallConsumer {
    fn get_connection(&self) -> Option<&Connection> {
        if self.conn.is_none() {
            None
        } else {
            Some(self.conn.as_ref().unwrap())
        }
    }

    fn get_channel(&self) -> Option<&Channel> {
        if self.channel.is_none() {
            None
        } else {
            Some(self.channel.as_ref().unwrap())
        }
    }

    fn set_connection(&mut self, conn: Connection) {
        self.conn = Some(conn);
    }

    fn set_channel(&mut self, channel: Channel) {
        self.channel = Some(channel)
    }
}

impl DmCallConsumer {
    pub fn new(db: Database, authifier_db: authifier::Database) -> DmCallConsumer {
        DmCallConsumer {
            db,
            authifier_db,
            conn: None,
            channel: None,
        }
    }

    async fn consume_event(
        &mut self,
        _channel: &Channel,
        _deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) -> Result<()> {
        let content = String::from_utf8(content)?;
        let _p: InternalDmCallPayload = serde_json::from_str(content.as_str())?;
        let payload = _p.payload;

        debug!("Received dm call start/stop event");

        let (revolt_database::Channel::DirectMessage { recipients, .. }
        | revolt_database::Channel::Group { recipients, .. }) =
            self.db.fetch_channel(&payload.channel_id).await?
        else {
            warn!(
                "Discarding dm call start/stop event for non-dm/group channel {}",
                payload.channel_id
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

        let config = revolt_config::config().await;

        for user_id in call_recipients {
            if let Ok(sessions) = self.authifier_db.find_sessions(&user_id).await {
                for session in sessions {
                    if let Some(sub) = session.subscription {
                        let mut sendable = PayloadToService {
                            notification: PayloadKind::DmCallStartEnd(payload.clone()),
                            token: sub.auth,
                            user_id: session.user_id,
                            session_id: session.id,
                            extras: HashMap::new(),
                        };

                        let args: BasicPublishArguments;

                        if sub.endpoint == "apn" {
                            args = BasicPublishArguments::new(
                                config.pushd.exchange.as_str(),
                                config.pushd.apn.queue.as_str(),
                            )
                            .finish();
                        } else if sub.endpoint == "fcm" {
                            args = BasicPublishArguments::new(
                                config.pushd.exchange.as_str(),
                                config.pushd.fcm.queue.as_str(),
                            )
                            .finish();
                        } else {
                            // web push (vapid)
                            args = BasicPublishArguments::new(
                                config.pushd.exchange.as_str(),
                                config.pushd.vapid.queue.as_str(),
                            )
                            .finish();
                            sendable.extras.insert("p265dh".to_string(), sub.p256dh);
                            sendable
                                .extras
                                .insert("endpoint".to_string(), sub.endpoint.clone());
                        }

                        let payload = serde_json::to_string(&sendable)?;

                        publish_message(self, payload.into(), args).await;
                    }
                }
            }
        }

        Ok(())
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AsyncConsumer for DmCallConsumer {
    /// This consumer handles delegating messages into their respective platform queues.
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        if let Err(err) = self
            .consume_event(channel, deliver, basic_properties, content)
            .await
        {
            revolt_config::capture_anyhow(&err);
            warn!("Failed to process dm call start/stop event: {err:?}");
        }
    }
}
