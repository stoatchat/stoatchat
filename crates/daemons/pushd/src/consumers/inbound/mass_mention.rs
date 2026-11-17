use std::{
    collections::HashSet,
    hash::RandomState,
    sync::Arc,
};

use crate::utils::{render_notification_content, Consumer};
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use stoat_otel::WideContextRepository;
use tracing::Instrument;
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
    /// Returns how many pushes were queued
    async fn fire_notification_for_users(
        &self,
        push: &PushNotification,
        users: &[String],
    ) -> Result<i64> {
        let mut queued = 0;
        if let Ok(sessions) = self
            .db
            .fetch_sessions_with_subscription(users)
            .await
        {
            for session in sessions {
                let notification = PayloadKind::MessageNotification(push.clone());
                if self.publish_push(notification, session).await? {
                    queued += 1;
                }
            }
        }

        Ok(queued)
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
    async fn consume(&self, delivery: Delivery, context: &WideContextRepository) -> Result<()> {
        let mut payload: MassMessageSentPayload = serde_json::from_slice(&delivery.data)?;
        context.add("server_id", payload.server_id.clone());
        context.add("notifications.count", payload.notifications.len() as i64);
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
        let mut queued = 0;
        let mut chunks = 0;
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

                        debug!(?userids, "Userids in chunk");

                        if let Err(err) = self
                            .db
                            .add_mention_to_many_unreads(push.channel.id(), &userids, &ack_chnl)
                            .await
                        {
                            error!(error = ?err, "Failed to add mass mention to unreads");
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

                        debug!(?target_users, ?online_users, "Userids after filter");

                        chunks += 1;
                        queued += self
                            .fire_notification_for_users(&push, &target_users)
                            .instrument(info_span!(
                                "mass_mention.chunk",
                                mention = "everyone",
                                members = userids.len(),
                                targets = target_users.len()
                            ))
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

                        debug!(?viewing_members, "Viewing members");

                        let online = revolt_presence::filter_online(&viewing_members).await;
                        debug!(?online, "Online members");

                        let targets: Vec<String> = viewing_members
                            .iter()
                            .filter(|m| !online.contains(*m))
                            .cloned()
                            .collect();

                        debug!(?targets, "Targets");

                        chunks += 1;
                        queued += self
                            .fire_notification_for_users(&push, &targets)
                            .instrument(info_span!(
                                "mass_mention.chunk",
                                mention = "roles",
                                members = chunk.len(),
                                targets = targets.len()
                            ))
                            .await?;
                    }
                }
            }
        }

        context.add("chunks.count", chunks);
        context.add("pushes.queued", queued);
        Ok(())
    }
}
