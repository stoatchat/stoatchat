use redis_kiss::{
    AsyncCommands, get_connection,
    redis::{SetExpiry, SetOptions},
};
use revolt_permissions::{ChannelPermission, calculate_channel_permissions};
use revolt_result::{Result, ToRevoltError};

use crate::{AMQP, Channel, Database, Server, User, events::client::EventV1};

const ACK_PENDING_TTL: usize = 15 * 60; // 15 seconds

pub async fn ack_channel(user: &str, channel: &str, message: &str, amqp: &AMQP) -> Result<()> {
    let mut redis = get_connection()
        .await
        .map_err(|_| create_error!(InternalError))?;

    let key = format!("acker:{user}+{channel}");

    let old: Option<String> = redis
        .set_options(
            &key,
            message,
            SetOptions::default()
                .get(true)
                .with_expiration(SetExpiry::EX(ACK_PENDING_TTL)),
        )
        .await
        .to_internal_error()?;

    #[allow(clippy::collapsible_if)]
    if old.is_none() {
        if let Err(e) = amqp
            .process_ack(user, Some(channel), None)
            .await
            .to_internal_error()
        {
            let _ = redis.del::<_, ()>(&key).await;
            return Err(e);
        }
    }

    Ok(())
}

pub async fn ack_server(user: &User, server: &Server, db: &Database, amqp: &AMQP) -> Result<()> {
    let mut redis = get_connection()
        .await
        .map_err(|_| create_error!(InternalError))?;

    let channels = db.fetch_channels(&server.channels).await?;
    let query = crate::util::permissions::DatabasePermissionQuery::new(db, user).server(server);

    for channel in channels {
        let channel_id = channel.id();
        let mut q = query.clone().channel(&channel);

        if calculate_channel_permissions(&mut q)
            .await
            .has_channel_permission(ChannelPermission::ViewChannel)
        {
            let channel_last_msg = match &channel {
                Channel::TextChannel {
                    last_message_id, ..
                } => last_message_id,
                _ => unreachable!(),
            }
            .clone();

            if let Some(channel_last_msg) = channel_last_msg {
                let key = format!("acker:{}+{}", user.id, channel_id);
                let old: Option<String> = redis
                    .set_options(
                        &key,
                        &channel_last_msg,
                        SetOptions::default()
                            .get(true)
                            .with_expiration(SetExpiry::EX(ACK_PENDING_TTL)),
                    )
                    .await
                    .to_internal_error()?;

                if old.is_none() {
                    if let Err(e) = amqp
                        .process_ack(&user.id, Some(channel_id), Some(&server.id))
                        .await
                        .to_internal_error()
                    {
                        let _ = redis.del::<_, ()>(&key).await;
                        return Err(e);
                    }

                    EventV1::ChannelAck {
                        id: channel_id.to_string(),
                        user: user.id.clone(),
                        message_id: channel_last_msg,
                    }
                    .p(user.id.clone())
                    .await;
                }
            }
        }
    }

    Ok(())
}
