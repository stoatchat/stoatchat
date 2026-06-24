// Queue Type: Debounced
use deadqueue::limited::Queue;
use once_cell::sync::Lazy;
use std::{collections::HashMap, time::Duration};

use crate::{Database, PartialChannel};

use super::DelayedTask;

/// Task information
struct Data {
    /// Channel to update
    channel: String,
    /// Latest message ID
    id: String,
    /// Whether the channel is a DM
    is_dm: bool,
}

/// Task information
#[derive(Debug)]
struct Task {
    /// Latest message ID
    id: String,
    /// Whether the channel is a DM
    is_dm: bool,
}

static Q: Lazy<Queue<Data>> = Lazy::new(|| Queue::new(10_000));

/// Queue a new task for a worker
pub async fn queue(channel: String, id: String, is_dm: bool) {
    Q.try_push(Data { channel, id, is_dm }).ok();
    info!("Queue is using {} slots from {}.", Q.len(), Q.capacity());
}

/// Start a new worker
pub async fn worker(db: Database) {
    let mut tasks = HashMap::<String, DelayedTask<Task>>::new();
    let mut keys = vec![];

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
                let Task { id, is_dm, .. } = task.data;

                let mut channel = PartialChannel {
                    last_message_id: Some(id.to_string()),
                    ..Default::default()
                };

                if is_dm {
                    channel.active = Some(true);
                }

                match db.update_channel(key, &channel, vec![]).await {
                    Ok(_) => info!("Updated last_message_id for {key} to {id}."),
                    Err(err) => error!("Failed to update last_message_id with {err:?}!"),
                }
            }
        }

        // Clear keys
        keys.clear();

        // Queue incoming tasks.
        while let Some(Data { channel, id, is_dm }) = Q.try_pop() {
            if let Some(task) = tasks.get_mut(&channel) {
                task.data.id = id;
                task.delay();
            } else {
                tasks.insert(channel, DelayedTask::new(Task { id, is_dm }));
            }
        }

        // Sleep for an arbitrary amount of time.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
