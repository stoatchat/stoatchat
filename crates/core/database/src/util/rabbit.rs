use amqprs::{channel::Channel, connection::Connection};
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap, future::ready, sync::{LazyLock, RwLock}, thread::{ThreadId, current}
};

static RABBIT_CONNECTION: OnceCell<Connection> = OnceCell::new();
static RABBIT_CHANNELS: LazyLock<RwLock<HashMap<ThreadId, Channel>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub async fn get_channel_with_init<F: AsyncFnOnce(Channel) -> Channel>(init: F) -> Channel {
    let conn = RABBIT_CONNECTION
        .get()
        .expect("Rabbit connection is not initialised.");

    let thread_id = current().id();

    let channel = RABBIT_CHANNELS
        .read()
        .expect("Channels poisioned")
        .get(&thread_id)
        .cloned();

    if let Some(channel) = channel {
        channel
    } else {
        let mut channel =
            conn.open_channel(None)
                .await
                .expect("Failed to open rabbitmq channel");

        channel = init(channel).await;

        RABBIT_CHANNELS
            .write()
            .expect("Channels poisioned")
            .insert(thread_id, channel.clone());
        channel
    }
}

pub async fn get_channel() -> Channel {
    get_channel_with_init(ready).await
}

pub fn set_rabbitmq_connection(connection: Connection) -> bool {
    RABBIT_CONNECTION.set(connection).is_ok()
}
