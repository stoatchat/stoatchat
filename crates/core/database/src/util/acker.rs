use redis_kiss::{get_connection, AsyncCommands};
use revolt_result::{Result, ToRevoltError};

use crate::AMQP;

pub async fn ack(user: &str, channel: &str, message: &str, amqp: &AMQP) -> Result<()> {
    let mut redis = get_connection()
        .await
        .map_err(|_| create_error!(InternalError))?;

    let old: Option<String> = redis
        .getset(format!("acker:{user}+{channel}"), message)
        .await
        .to_internal_error()?;

    info!("old state: {:?}", old);

    if old.is_none() || old.unwrap() == message {
        amqp.process_ack(user, channel).await.to_internal_error()?;
    }

    Ok(())
}
