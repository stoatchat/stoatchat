use redis_kiss::{get_connection, AsyncCommands};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{Result, ToRevoltError};

use crate::{events::client::EventV1, Channel, Database, Server, User, AMQP};

pub async fn ack_channel(user: &str, channel: &str, message: &str, amqp: &AMQP) -> Result<()> {
    let mut redis = get_connection()
        .await
        .map_err(|_| create_error!(InternalError))?;

    let old: Option<String> = redis
        .getset(format!("acker:{user}+{channel}"), message)
        .await
        .to_internal_error()?;

    if old.is_none() || old.unwrap() == message {
        amqp.process_ack(user, Some(channel), None)
            .await
            .to_internal_error()?;
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
                let old: Option<String> = redis
                    .getset(
                        format!("acker:{}+{}", user.id, channel_id),
                        &channel_last_msg,
                    )
                    .await
                    .to_internal_error()?;

                if old.is_none() || old.unwrap() == channel_last_msg {
                    amqp.process_ack(&user.id, Some(channel_id), Some(&server.id))
                        .await
                        .to_internal_error()?;

                    EventV1::ChannelAck {
                        id: channel_id.to_string(),
                        user: user.id.clone(),
                        message_id: channel_last_msg,
                    }
                    .private(user.id.clone())
                    .await;
                }
            }
        }
    }

    Ok(())
}
