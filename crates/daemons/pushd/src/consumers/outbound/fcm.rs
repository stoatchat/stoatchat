use std::{collections::HashMap, time::Duration};

use amqprs::{channel::Channel as AmqpChannel, consumer::AsyncConsumer, BasicProperties, Deliver};

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use fcm_v1::{
    auth::{Authenticator, ServiceAccountKey},
    message::Message,
    Client, Error as FcmError,
};
use revolt_config::config;
use revolt_database::{events::rabbit::*, Database};
use revolt_models::v0::{Channel, PushNotification};
use serde_json::Value;

/// Custom notification data
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationData {
    FRReceived {
        id: String,
        username: String,
    },
    FRAccepted {
        id: String,
        username: String,
    },
    Generic {
        title: String,
        body: String,
        image: Option<String>,
    },
    Message {
        title: String,
        body: String,
        image: String,
        tag: String,
    },
    DmCallStartEnd {
        initiator_id: String,
        channel_id: String,
        started_at: String,
        ended: bool,
        duration: usize,
    },
}

impl NotificationData {
    pub fn get_type(&self) -> &str {
        match self {
            NotificationData::FRReceived { .. } => "push.fr.receive",
            NotificationData::FRAccepted { .. } => "push.fr.accept",
            NotificationData::Generic { .. } => "push.generic",
            NotificationData::Message { .. } => "push.message",
            NotificationData::DmCallStartEnd { .. } => "push.dm.call",
        }
    }

    pub fn into_payload(self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "type".to_string(),
            Value::String(self.get_type().to_string()),
        );

        match self {
            NotificationData::FRReceived { id, username } => {
                data.insert("id".to_string(), Value::String(id));
                data.insert("username".to_string(), Value::String(username));
            }
            NotificationData::FRAccepted { id, username } => {
                data.insert("id".to_string(), Value::String(id));
                data.insert("username".to_string(), Value::String(username));
            }
            NotificationData::Generic { title, body, image } => {
                data.insert("title".to_string(), Value::String(title));
                data.insert("body".to_string(), Value::String(body));

                if let Some(image) = image {
                    data.insert("image".to_string(), Value::String(image));
                }
            }
            NotificationData::Message {
                title,
                body,
                image,
                tag,
            } => {
                data.insert("title".to_string(), Value::String(title));
                data.insert("body".to_string(), Value::String(body));
                data.insert("image".to_string(), Value::String(image));
                data.insert("tag".to_string(), Value::String(tag));
            }
            NotificationData::DmCallStartEnd {
                initiator_id,
                channel_id,
                started_at,
                ended,
                duration,
            } => {
                data.insert("initiator_id".to_string(), Value::String(initiator_id));
                data.insert("channel_id".to_string(), Value::String(channel_id));
                data.insert("started_at".to_string(), Value::String(started_at));
                data.insert("ended".to_string(), Value::Bool(ended));
                data.insert("duration".to_string(), Value::Number(duration.into()));
            }
        }

        data
    }
}

pub struct FcmOutboundConsumer {
    db: Database,
    client: Client,
}

impl FcmOutboundConsumer {
    fn format_title(&self, notification: &PushNotification) -> String {
        // ideally this changes depending on context
        // in a server, it would look like "Sendername, #channelname in servername"
        // in a group, it would look like "Sendername in groupname"
        // in a dm it should just be "Sendername".
        // not sure how feasible all those are given the PushNotification object as it currently stands.

        #[allow(deprecated)]
        match &notification.channel {
            Channel::DirectMessage { .. } => notification.author.clone(),
            Channel::Group { name, .. } => format!("{}, #{}", notification.author, name),
            Channel::TextChannel { name, .. } => {
                format!("{} in #{}", notification.author, name)
            }
            _ => "Unknown".to_string(),
        }
    }
}

impl FcmOutboundConsumer {
    pub async fn new(db: Database) -> Result<FcmOutboundConsumer, &'static str> {
        let config = revolt_config::config().await;

        Ok(FcmOutboundConsumer {
            db,
            client: Client::new(
                Authenticator::service_account::<&str>(ServiceAccountKey {
                    key_type: Some(config.pushd.fcm.key_type),
                    project_id: Some(config.pushd.fcm.project_id.clone()),
                    private_key_id: Some(config.pushd.fcm.private_key_id),
                    private_key: config.pushd.fcm.private_key,
                    client_email: config.pushd.fcm.client_email,
                    client_id: Some(config.pushd.fcm.client_id),
                    auth_uri: Some(config.pushd.fcm.auth_uri),
                    token_uri: config.pushd.fcm.token_uri,
                    auth_provider_x509_cert_url: Some(config.pushd.fcm.auth_provider_x509_cert_url),
                    client_x509_cert_url: Some(config.pushd.fcm.client_x509_cert_url),
                })
                .await
                .unwrap(),
                config.pushd.fcm.project_id,
                false,
                Duration::from_secs(5),
            ),
        })
    }

    async fn consume_event(
        &mut self,
        _channel: &AmqpChannel,
        _deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) -> Result<()> {
        let content = String::from_utf8(content)?;
        let payload: PayloadToService = serde_json::from_str(content.as_str())?;

        #[allow(clippy::needless_late_init)]
        let resp: Result<Message, FcmError>;

        match payload.notification {
            PayloadKind::FRReceived(alert) => {
                let name = alert
                    .from_user
                    .display_name
                    .or(Some(format!(
                        "{}#{}",
                        alert.from_user.username, alert.from_user.discriminator
                    )))
                    .clone()
                    .ok_or_else(|| anyhow!("missing name"))?;

                let data = NotificationData::FRReceived {
                    id: alert.from_user.id,
                    username: name,
                };

                let msg = Message {
                    token: Some(payload.token),
                    data: Some(data.into_payload()),
                    ..Default::default()
                };

                resp = self.client.send(&msg).await;
            }

            PayloadKind::FRAccepted(alert) => {
                let name = alert
                    .accepted_user
                    .display_name
                    .or(Some(format!(
                        "{}#{}",
                        alert.accepted_user.username, alert.accepted_user.discriminator
                    )))
                    .clone()
                    .ok_or_else(|| anyhow!("missing name"))?;

                let data = NotificationData::FRAccepted {
                    id: alert.accepted_user.id,
                    username: name,
                };

                let msg = Message {
                    token: Some(payload.token),
                    data: Some(data.into_payload()),
                    ..Default::default()
                };

                resp = self.client.send(&msg).await;
            }
            PayloadKind::Generic(alert) => {
                let data = NotificationData::Generic {
                    title: alert.title,
                    body: alert.body,
                    image: alert.icon,
                };

                let msg = Message {
                    token: Some(payload.token),
                    data: Some(data.into_payload()),
                    ..Default::default()
                };

                resp = self.client.send(&msg).await;
            }

            PayloadKind::MessageNotification(alert) => {
                let data = NotificationData::Message {
                    title: self.format_title(&alert),
                    body: alert.body,
                    image: alert.icon,
                    tag: alert.tag,
                };

                let msg = Message {
                    token: Some(payload.token),
                    data: Some(data.into_payload()),
                    ..Default::default()
                };

                resp = self.client.send(&msg).await;
            }

            PayloadKind::DmCallStartEnd(alert) => {
                let data = NotificationData::DmCallStartEnd {
                    initiator_id: alert.initiator_id,
                    channel_id: alert.channel_id,
                    started_at: alert.started_at.unwrap_or_else(|| "".to_string()),
                    ended: alert.ended,
                    duration: config().await.api.livekit.call_ring_duration,
                };

                let msg = Message {
                    token: Some(payload.token),
                    data: Some(data.into_payload()),
                    ..Default::default()
                };

                resp = self.client.send(&msg).await;
            }

            PayloadKind::BadgeUpdate(_) => {
                bail!("FCM cannot handle badge updates and they should not be sent here.");
            }
        }

        if let Err(err) = resp {
            match err {
                FcmError::Auth => {
                    if let Err(err) = self
                        .db
                        .remove_push_subscription_by_session_id(&payload.session_id)
                        .await
                    {
                        revolt_config::capture_error(&err);
                    }
                }
                err => {
                    revolt_config::capture_error(&err);
                }
            }
        }

        Ok(())
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AsyncConsumer for FcmOutboundConsumer {
    async fn consume(
        &mut self,
        channel: &AmqpChannel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        if let Err(err) = self
            .consume_event(channel, deliver, basic_properties, content)
            .await
        {
            revolt_config::capture_anyhow(&err);
            eprintln!("Failed to process FCM event: {err:?}");
        }
    }
}
