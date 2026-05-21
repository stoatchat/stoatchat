use std::{borrow::Cow, collections::BTreeMap, io::Cursor, sync::Arc};

use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use base64::{
    engine::{self},
    Engine as _,
};
use lapin::{message::Delivery, Channel as AMQPChannel, Connection};
use revolt_a2::{
    request::{
        notification::{DefaultAlert, NotificationOptions},
        payload::{APSAlert, APSSound, Payload, PayloadLike, APS},
    },
    Client, ClientConfig, Endpoint, Error, ErrorBody, ErrorReason, Priority, PushType, Response,
};
use revolt_database::{events::rabbit::*, Database};
use revolt_models::v0::{Channel, Message, PushNotification};
use serde::Serialize;

// region: payload

#[derive(Serialize, Debug)]
struct MessagePayload<'a> {
    aps: APS<'a>,
    #[serde(skip_serializing)]
    options: NotificationOptions<'a>,
    #[serde(skip_serializing)]
    device_token: &'a str,

    message: &'a Message,
    url: &'a str,
    #[serde(rename = "camelCase")]
    author_avatar: &'a str,
    #[serde(rename = "camelCase")]
    author_display_name: &'a str,
    #[serde(rename = "camelCase")]
    channel_name: &'a str,
}

impl<'a> PayloadLike for MessagePayload<'a> {
    fn get_device_token(&self) -> &'a str {
        self.device_token
    }
    fn get_options(&self) -> &NotificationOptions<'a> {
        &self.options
    }
}

#[derive(Serialize, Debug)]
struct CallStartStopPayload<'a> {
    aps: APS<'a>,
    #[serde(skip_serializing)]
    options: NotificationOptions<'a>,
    #[serde(skip_serializing)]
    device_token: &'a str,

    initiator_id: &'a str,
    #[serde(rename = "camelCase")]
    channel_id: &'a str,
    #[serde(rename = "camelCase")]
    started_at: &'a str,
    #[serde(rename = "camelCase")]
    ended: bool,
}

impl<'a> PayloadLike for CallStartStopPayload<'a> {
    fn get_device_token(&self) -> &'a str {
        self.device_token
    }
    fn get_options(&self) -> &NotificationOptions<'a> {
        &self.options
    }
}

// region: consumer

#[derive(Clone)]
#[allow(unused)]
pub struct ApnsOutboundConsumer {
    db: Database,
    authifier_db: authifier::Database,
    connection: Arc<Connection>,
    channel: Arc<AMQPChannel>,
    client: Client,
}

impl ApnsOutboundConsumer {
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

    async fn get_badge_count(&self, user: &str) -> Option<u32> {
        if let Ok(unreads) = self.db.fetch_unread_mentions(user).await {
            let mut mention_count = 0;
            for channel in unreads {
                if let Some(mentions) = channel.mentions {
                    mention_count += mentions.len() as u32
                }
            }

            debug!("Got badge count for APN: {}", mention_count);

            return Some(mention_count);
        }
        None
    }
}

#[async_trait]
impl Consumer for ApnsOutboundConsumer {
    async fn create(
        db: Database,
        authifier_db: authifier::Database,
        connection: Arc<Connection>,
        channel: Arc<AMQPChannel>,
    ) -> Self {
        let config = revolt_config::config().await;

        if config.pushd.apn.pkcs8.is_empty()
            || config.pushd.apn.key_id.is_empty()
            || config.pushd.apn.team_id.is_empty()
        {
            panic!("Missing APN keys.");
        }

        let endpoint = if config.pushd.apn.sandbox {
            Endpoint::Sandbox
        } else {
            Endpoint::Production
        };

        let pkcs8 = engine::general_purpose::STANDARD
            .decode(config.pushd.apn.pkcs8.clone())
            .expect("valid `pcks8`");

        let client_config = ClientConfig::new(endpoint);

        let client = Client::token(
            &mut Cursor::new(pkcs8),
            config.pushd.apn.key_id.clone(),
            config.pushd.apn.team_id.clone(),
            client_config,
        )
        .expect("could not create APN client");

        Self {
            db,
            authifier_db,
            connection,
            channel,
            client,
        }
    }

    fn channel(&self) -> &Arc<AMQPChannel> {
        &self.channel
    }

    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let payload: PayloadToService = serde_json::from_slice(&delivery.data)?;

        let payload_options = NotificationOptions {
            apns_id: None,
            apns_push_type: Some(PushType::Alert),
            apns_expiration: None,
            apns_priority: Some(Priority::High),
            apns_topic: Some("chat.revolt.app"),
            apns_collapse_id: None,
        };

        let resp = match payload.notification {
            PayloadKind::FRReceived(alert) => {
                let loc_args = vec![Cow::from(
                    alert.from_user.display_name.clone().unwrap_or_else(|| {
                        format!(
                            "{}#{}",
                            alert.from_user.username, alert.from_user.discriminator
                        )
                    }),
                )];

                let apn_payload = Payload {
                    aps: APS {
                        alert: Some(APSAlert::Default(DefaultAlert {
                            title: None,
                            subtitle: None,
                            body: None,
                            title_loc_key: None,
                            title_loc_args: None,
                            action_loc_key: None,
                            loc_key: Some("push.fr.received"),
                            loc_args: Some(loc_args),
                            launch_image: None,
                        })),
                        badge: self.get_badge_count(&payload.user_id).await,
                        sound: Some(APSSound::Sound("default")),
                        thread_id: None,
                        content_available: None,
                        category: None,
                        mutable_content: Some(1),
                        url_args: None,
                    },
                    device_token: &payload.token,
                    options: payload_options.clone(),
                    data: BTreeMap::new(),
                };

                debug!(
                    "Sending friend request received for user: {:}",
                    &payload.user_id
                );
                self.client.send(apn_payload).await
            }

            PayloadKind::FRAccepted(alert) => {
                let loc_args = vec![Cow::from(
                    alert.accepted_user.display_name.clone().unwrap_or_else(|| {
                        format!(
                            "{}#{}",
                            alert.accepted_user.username, alert.accepted_user.discriminator
                        )
                    }),
                )];

                let apn_payload = Payload {
                    aps: APS {
                        alert: Some(APSAlert::Default(DefaultAlert {
                            title: None,
                            subtitle: None,
                            body: None,
                            title_loc_key: None,
                            title_loc_args: None,
                            action_loc_key: None,
                            loc_key: Some("push.fr.accepted"),
                            loc_args: Some(loc_args),
                            launch_image: None,
                        })),
                        badge: self.get_badge_count(&payload.user_id).await,
                        sound: Some(APSSound::Sound("default")),
                        thread_id: None,
                        content_available: None,
                        category: None,
                        mutable_content: Some(1),
                        url_args: None,
                    },
                    device_token: &payload.token,
                    options: payload_options.clone(),
                    data: BTreeMap::new(),
                };

                debug!(
                    "Sending friend request accept for user: {:}",
                    &payload.user_id
                );
                self.client.send(apn_payload).await
            }
            PayloadKind::Generic(alert) => {
                let apn_payload = Payload {
                    aps: APS {
                        alert: Some(APSAlert::Default(DefaultAlert {
                            title: Some(&alert.title),
                            subtitle: None,
                            body: Some(&alert.body),
                            title_loc_key: None,
                            title_loc_args: None,
                            action_loc_key: None,
                            loc_key: None,
                            loc_args: None,
                            launch_image: None,
                        })),
                        badge: self.get_badge_count(&payload.user_id).await,
                        sound: Some(APSSound::Sound("default")),
                        thread_id: None,
                        content_available: None,
                        category: None,
                        mutable_content: Some(1),
                        url_args: None,
                    },
                    device_token: &payload.token,
                    options: payload_options.clone(),
                    data: BTreeMap::new(),
                };

                debug!(
                    "Sending generic notification for user: {:}",
                    &payload.user_id
                );
                self.client.send(apn_payload).await
            }

            PayloadKind::MessageNotification(alert) => {
                let title = self.format_title(&alert);
                let apn_payload = MessagePayload {
                    aps: APS {
                        alert: Some(APSAlert::Default(DefaultAlert {
                            title: Some(&title),
                            subtitle: None,
                            body: Some(&alert.body),
                            title_loc_key: None,
                            title_loc_args: None,
                            action_loc_key: None,
                            loc_key: None,
                            loc_args: None,
                            launch_image: None,
                        })),
                        badge: self.get_badge_count(&payload.user_id).await,
                        sound: Some(APSSound::Sound("default")),
                        thread_id: Some(alert.channel.id()),
                        content_available: None,
                        category: None,
                        mutable_content: Some(1),
                        url_args: None,
                    },
                    device_token: &payload.token,
                    options: payload_options.clone(),
                    message: &alert.message,
                    url: &alert.url,
                    author_avatar: &alert.icon,
                    author_display_name: &alert.author,
                    channel_name: alert.channel.name().unwrap_or(&title),
                };

                debug!(
                    "Sending message notification for user: {:}",
                    &payload.user_id
                );
                self.client.send(apn_payload).await
            }

            PayloadKind::BadgeUpdate(badge) => {
                let apn_payload = Payload {
                    aps: APS {
                        badge: Some(badge as u32),
                        ..Default::default()
                    },
                    device_token: &payload.token,
                    options: payload_options.clone(),
                    data: BTreeMap::new(),
                };

                debug!("Sending badge update for user: {:}", &payload.user_id);
                self.client.send(apn_payload).await
            }

            PayloadKind::DmCallStartEnd(alert) => {
                let started_at = alert.started_at.map_or(String::new(), |f| f.clone());

                let apn_payload = CallStartStopPayload {
                    aps: APS {
                        alert: None,
                        badge: self.get_badge_count(&payload.user_id).await,
                        sound: None,
                        thread_id: None,
                        content_available: None,
                        category: None,
                        mutable_content: Some(1),
                        url_args: None,
                    },
                    device_token: &payload.token,
                    options: payload_options.clone(),
                    initiator_id: &alert.initiator_id,
                    channel_id: &alert.channel_id,
                    started_at: &started_at,
                    ended: alert.ended,
                };

                debug!(
                    "Sending call start/stop notification for user: {:}",
                    &payload.user_id
                );
                self.client.send(apn_payload).await
            }
        };

        match resp {
            Err(Error::ResponseError(Response {
                error:
                    Some(ErrorBody {
                        reason: ErrorReason::BadDeviceToken | ErrorReason::Unregistered,
                        ..
                    }),
                ..
            })) => {
                info!(
                    "Removing APNS subscription id {:} (user: {:}) due to invalid token",
                    &payload.session_id, &payload.user_id
                );

                if let Err(err) = self
                    .db
                    .remove_push_subscription_by_session_id(&payload.session_id)
                    .await
                {
                    revolt_config::capture_error(&err);
                }
            }
            resp => {
                resp?;
            }
        };

        Ok(())
    }
}
