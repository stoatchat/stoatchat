use std::collections::HashMap;

use amqprs::{channel::Channel as AmqpChannel, consumer::AsyncConsumer, BasicProperties, Deliver};

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use base64::{
    engine::{self},
    Engine as _,
};
use revolt_database::{events::rabbit::*, util::format_display_name, Database};
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, SubscriptionKeys, VapidSignatureBuilder,
    WebPushClient, WebPushError, WebPushMessageBuilder,
};

pub struct VapidOutboundConsumer {
    db: Database,
    client: IsahcWebPushClient,
    pkey: Vec<u8>,
}

impl VapidOutboundConsumer {
    pub async fn new(db: Database) -> Result<VapidOutboundConsumer> {
        let config = revolt_config::config().await;

        if config.pushd.vapid.private_key.is_empty() | config.pushd.vapid.public_key.is_empty() {
            bail!("no Vapid keys present");
        }

        let web_push_private_key = engine::general_purpose::URL_SAFE_NO_PAD
            .decode(config.pushd.vapid.private_key)
            .expect("valid `VAPID_PRIVATE_KEY`");

        Ok(VapidOutboundConsumer {
            db,
            client: IsahcWebPushClient::new().unwrap(),
            pkey: web_push_private_key,
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

        let subscription = SubscriptionInfo {
            endpoint: payload
                .extras
                .get("endpoint")
                .ok_or_else(|| anyhow!("missing endpoint"))?
                .clone(),
            keys: SubscriptionKeys {
                auth: payload.token,
                p256dh: payload
                    .extras
                    .get("p256dh")
                    .ok_or_else(|| anyhow!("missing p256dh"))?
                    .clone(),
            },
        };

        #[allow(clippy::needless_late_init)]
        let payload_body: String;

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

                let mut body = HashMap::new();
                body.insert("body", format!("{} sent you a friend request", name));

                payload_body = serde_json::to_string(&body)?;
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

                let mut body = HashMap::new();
                body.insert("body", format!("{} accepted your friend request", name));

                payload_body = serde_json::to_string(&body)?;
            }
            PayloadKind::Generic(alert) => {
                payload_body = serde_json::to_string(&alert)?;
            }
            PayloadKind::MessageNotification(alert) => {
                payload_body = serde_json::to_string(&alert)?;
            }
            PayloadKind::DmCallStartEnd(alert) => {
                let initiator_name = if let Some(server_id) =
                    self.db.fetch_channel(&alert.channel_id).await?.server()
                {
                    format_display_name(&self.db, &alert.initiator_id, Some(server_id)).await
                } else {
                    format_display_name(&self.db, &alert.initiator_id, None).await
                }?;

                let channel = self.db.fetch_channel(&alert.channel_id).await?;
                let mut body = HashMap::new();

                match channel {
                    revolt_database::Channel::DirectMessage { .. } => {
                        body.insert("body", format!("{} is calling you", initiator_name));
                    }
                    revolt_database::Channel::Group { name, .. } => {
                        body.insert(
                            "body",
                            format!("{} is calling your group, {}", initiator_name, name),
                        );
                    }
                    _ => bail!("Invalid DmCallStart/End channel type"),
                }

                payload_body = serde_json::to_string(&body)?;
            }
            PayloadKind::BadgeUpdate(_) => {
                bail!("Vapid cannot handle badge updates and they should not be sent here.");
            }
        }

        match VapidSignatureBuilder::from_pem(std::io::Cursor::new(&self.pkey), &subscription) {
            Ok(sig_builder) => match sig_builder.build() {
                Ok(signature) => {
                    let mut builder = WebPushMessageBuilder::new(&subscription);
                    builder.set_vapid_signature(signature);

                    builder.set_payload(ContentEncoding::AesGcm, payload_body.as_bytes());

                    match builder.build() {
                        Ok(msg) => {
                            if let Err(err) = self.client.send(msg).await {
                                if err == WebPushError::Unauthorized {
                                    self.db
                                        .remove_push_subscription_by_session_id(&payload.session_id)
                                        .await?;
                                }
                            }

                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err.into()),
        }
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AsyncConsumer for VapidOutboundConsumer {
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
            eprintln!("Failed to process Vapid event: {err:?}");
        }
    }
}
