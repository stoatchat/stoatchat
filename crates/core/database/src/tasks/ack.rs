// Queue Type: Debounced
use crate::{AMQP, Database};

use deadqueue::limited::Queue;
use once_cell::sync::Lazy;

use std::{collections::HashMap, time::Duration};

use revolt_result::Result;

use super::DelayedTask;

/// Enumeration of possible events
#[derive(Debug, Eq, PartialEq)]
pub enum AckEvent {
    /// Acknowledge message in a channel for a user
    AckMessage {
        /// Message ID
        id: String,
    },
}

/// Task information
struct Data {
    /// Channel to ack
    channel: String,
    /// User to ack for
    user: Option<String>,
    /// Event
    event: AckEvent,
}

#[derive(Debug)]
struct Task {
    event: AckEvent,
}

static Q: Lazy<Queue<Data>> = Lazy::new(|| Queue::new(10_000));

/// Send pushes once the channel has been quiet for this long
const PROCESS_MESSAGE_SAVE_AFTER: Duration = Duration::from_secs(1);

/// but never hold them back longer than this
const PROCESS_MESSAGE_EXPIRE_AFTER: Duration = Duration::from_secs(3);

/// Queue a new task for a worker
pub async fn queue_ack(channel: String, user: String, event: AckEvent) {
    Q.try_push(Data {
        channel,
        user: Some(user),
        event,
    })
    .ok();

    info!(
        "Queue is using {} slots from {}. Queued type: ACK",
        Q.len(),
        Q.capacity()
    );
}

pub async fn handle_ack_event(
    event: &AckEvent,
    db: &Database,
    amqp: &AMQP,
    user: &Option<String>,
    channel: &str,
) -> Result<()> {
    match &event {
        #[allow(clippy::disallowed_methods)] // event is sent by higher level function
        AckEvent::AckMessage { id } => {
            let user = user.as_ref().unwrap();
            let user: &str = user.as_str();

            let unread = db.fetch_unread(user, channel).await?;
            let updated = db.acknowledge_message(channel, user, id).await?;

            if let (Some(before), Some(after)) = (unread, updated) {
                let before_mentions = before.mentions.unwrap_or_default().len();
                let after_mentions = after.mentions.unwrap_or_default().len();

                let mentions_acked = before_mentions.saturating_sub(after_mentions);

                if mentions_acked > 0 {
                    if let Err(err) = amqp
                        .ack_notification_message(
                            user.to_string(),
                            channel.to_string(),
                            id.to_owned(),
                        )
                        .await
                    {
                        revolt_config::capture_error(&err);
                    }
                };
            }
        }
    };

    Ok(())
}

/// Start a new worker
pub async fn worker(db: Database, amqp: AMQP) {
    let mut tasks = HashMap::<(Option<String>, String, u8), DelayedTask<Task>>::new();
    let mut keys: Vec<(Option<String>, String, u8)> = vec![];

    loop {
        // Find due tasks.
        for (key, task) in &tasks {
            if task.should_run() {
                keys.push(key.clone());
            }
        }

        // Commit any due tasks to the database.
        for key in &keys {
            if let Some(task) = tasks.remove(key) {
                let Task { event } = task.data;
                let (user, channel, _) = key;

                if let Err(err) = handle_ack_event(&event, &db, &amqp, user, channel).await {
                    revolt_config::capture_error(&err);
                    error!("{err:?} for {event:?}. ({user:?}, {channel})");
                } else {
                    info!("User {user:?} ack in {channel} with {event:?}");
                }
            }
        }

        // Clear keys
        keys.clear();

        // Queue incoming tasks.
        while let Some(Data {
            channel,
            user,
            mut event,
        }) = Q.try_pop()
        {
            info!("Took next ack from queue, now {} remaining", Q.len());

            let key: (Option<String>, String, u8) = (
                user,
                channel,
                match &event {
                    AckEvent::AckMessage { .. } => 0,
                },
            );
            if let Some(task) = tasks.get_mut(&key) {
                match &mut event {
                    AckEvent::AckMessage { .. } => {
                        // replace the last acked message with the new acked message
                        task.data.event = event;
                        task.delay();
                    }
                }
            } else {
                let task = match &event {
                    AckEvent::AckMessage { .. } => DelayedTask::new(Task { event }),
                };

                tasks.insert(key, task);
            }
        }

        // Sleep for an arbitrary amount of time.
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
