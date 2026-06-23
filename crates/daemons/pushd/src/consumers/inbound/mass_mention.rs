use std::{
    collections::{HashMap, HashSet},
    hash::RandomState,
    sync::Arc,
};

use crate::utils::{render_notification_content, Consumer};
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use revolt_database::{
    events::rabbit::*, util::bulk_permissions::BulkDatabasePermissionQuery, Database, Member,
    MessageFlagsValue,
};
use revolt_models::v0::{MessageFlags, PushNotification};
use revolt_result::ToRevoltError;

#[derive(Clone)]
#[allow(unused)]
pub struct MassMessageConsumer {
    db: Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

impl MassMessageConsumer {
    async fn fire_notification_for_users(
        &self,
        push: &PushNotification,
        users: &[String],
    ) -> Result<()> {
        if let Ok(sessions) = self
            .db
            .fetch_sessions_with_subscription(users)
            .await
        {
            let config = revolt_config::config().await;
            for session in sessions {
                if let Some(sub) = session.subscription {
                    let mut sendable = PayloadToService {
                        notification: PayloadKind::MessageNotification(push.clone()),
                        token: sub.auth,
                        user_id: session.user_id,
                        session_id: session.id,
                        extras: HashMap::new(),
                    };

                    let routing_key = match sub.endpoint.as_str() {
                        "apn" => &config.pushd.apn.queue,
                        "fcm" => &config.pushd.fcm.queue,
                        endpoint => {
                            sendable.extras.insert("p256dh".to_string(), sub.p256dh);
                            sendable
                                .extras
                                .insert("endpoint".to_string(), endpoint.to_string());

                            &config.pushd.vapid.queue
                        }
                    };

                    let payload = serde_json::to_string(&sendable)?;

                    self.publish_message(payload.as_bytes(), &config.pushd.exchange, routing_key)
                        .await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Consumer for MassMessageConsumer {
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

    /// This consumer handles adding mentions for all the users affected by a mass mention ping, and then sends out push notifications.
    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let mut payload: MassMessageSentPayload = serde_json::from_slice(&delivery.data)?;
        let config = revolt_config::config().await;

        for push in payload.notifications.iter_mut() {
            if let Ok(body) = render_notification_content(push, &self.db)
                .await
                .to_internal_error()
            {
                push.raw_body = Some(push.body.clone());
                push.body = body;
            }
        }

        debug!("Received mass message event");

        // We should only ever receive clumped messages from a single channel, so it's safe to reuse this many times.
        let mut query: Option<BulkDatabasePermissionQuery<'_>> = None;
        let query_db = self.db.clone();

        for push in payload.notifications {
            if query.is_none() {
                query = Some(
                    BulkDatabasePermissionQuery::from_server_id(&query_db, &payload.server_id)
                        .await
                        .from_channel_id(push.channel.id().to_string()) // wrong channel model, so fetch the right one
                        .await,
                );
            }

            let existing_mentions: HashSet<String, RandomState> =
                if let Some(ref mentions) = push.message.mentions {
                    HashSet::from_iter(mentions.iter().cloned())
                } else {
                    HashSet::new()
                };

            // KNOWN QUIRK: if you mention @online and role(s), the offline members with the role(s) wont get pinged
            if let Some(ref query) = query {
                let flags = MessageFlagsValue(push.message.flags);
                if flags.has(MessageFlags::MentionsEveryone) {
                    let mut db_query = self
                        .db
                        .fetch_all_members_chunked(&payload.server_id)
                        .await?;

                    let mut exhausted = false;
                    let ack_chnl = vec![push.channel.id().to_string()];
                    loop {
                        let mut chunk: Vec<Member> = vec![];
                        for _ in 0..config.pushd.mass_mention_chunk_size {
                            if let Some(member) = db_query.next().await {
                                chunk.push(member);
                            } else {
                                exhausted = true;
                                break;
                            }
                        }

                        let userids: Vec<String> =
                            chunk.iter().map(|member| member.id.user.clone()).collect();

                        debug!("Userids in chunk: {:?}", userids);

                        if let Err(err) = self
                            .db
                            .add_mention_to_many_unreads(push.channel.id(), &userids, &ack_chnl)
                            .await
                        {
                            revolt_config::capture_error(&err);
                        }

                        // ignore anyone in this list
                        let online_users = revolt_presence::filter_online(&userids).await;
                        let target_users: Vec<String> = userids
                            .iter()
                            .filter(|id| {
                                !online_users.contains(*id) && !existing_mentions.contains(*id)
                            })
                            .cloned()
                            .collect();

                        debug!(
                            "Userids after filter: {:?} (online: {:?}",
                            target_users, online_users
                        );

                        self.fire_notification_for_users(&push, &target_users)
                            .await?;

                        if exhausted {
                            break;
                        }
                    }
                } else if let Some(roles) = &push.message.role_mentions {
                    // role mentions
                    let mut role_members = self
                        .db
                        .fetch_all_members_with_roles_chunked(&payload.server_id, roles)
                        .await?;

                    let mut chunk = vec![];
                    let mut exhausted = false;

                    while !exhausted {
                        chunk.clear();

                        for _ in 0..config.pushd.mass_mention_chunk_size {
                            if let Some(member) = role_members.next().await {
                                chunk.push(member);
                            } else {
                                exhausted = true;
                                break;
                            }
                        }

                        let mut q = query.clone().members(&chunk);
                        let viewing_members: Vec<String> = q
                            .members_can_see_channel()
                            .await
                            .iter()
                            .filter_map(|(uid, viewable)| {
                                if *viewable && !existing_mentions.contains(uid) {
                                    Some(uid.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        debug!("viewing members: {:?}", viewing_members);

                        let online = revolt_presence::filter_online(&viewing_members).await;
                        debug!("online: {:?}", online);

                        let targets: Vec<String> = viewing_members
                            .iter()
                            .filter(|m| !online.contains(*m))
                            .cloned()
                            .collect();

                        debug!("targets: {:?}", targets);

                        self.fire_notification_for_users(&push, &targets).await?;
                    }
                }
            }
        }

        Ok(())
    }
}
