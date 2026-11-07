//! Semi-important background task management

use crate::{Database, AMQP};

use tokio::task;
use std::time::{Duration, Instant};

const WORKER_COUNT: usize = 5;

pub mod ack;
pub mod last_message_id;
pub mod process_embeds;

/// Spawn background workers
pub fn start_workers(db: Database, amqp: AMQP) {
    for _ in 0..WORKER_COUNT {
        task::spawn(ack::worker(db.clone(), amqp.clone()));
        task::spawn(last_message_id::worker(db.clone()));
        task::spawn(process_embeds::worker(db.clone()));
    }
}

/// Task with additional information on when it should run
pub struct DelayedTask<T> {
    pub data: T,
    run_now: bool,
    last_updated: Instant,
    first_seen: Instant,
    save_after: Duration,
    expire_after: Duration,
}

/// Commit to database every 30 seconds if the task is particularly active.
static EXPIRE_CONSTANT: Duration = Duration::from_secs(30);

/// Otherwise, commit to database after 5 seconds.
static SAVE_CONSTANT: Duration = Duration::from_secs(5);

impl<T> DelayedTask<T> {
    /// Create a new delayed task
    pub fn new(data: T) -> Self {
        Self::with_timings(data, SAVE_CONSTANT, EXPIRE_CONSTANT)
    }

    /// Create a new delayed task with custom timings
    pub fn with_timings(data: T, save_after: Duration, expire_after: Duration) -> Self {
        DelayedTask {
            data,
            run_now: false,
            last_updated: Instant::now(),
            first_seen: Instant::now(),
            save_after,
            expire_after,
        }
    }

    /// Push a task further back in time
    pub fn delay(&mut self) {
        self.last_updated = Instant::now()
    }

    /// Flag the task to run right away, regardless of the time
    pub fn run_immediately(&mut self) {
        self.run_now = true
    }

    /// Check if a task should run yet
    pub fn should_run(&self) -> bool {
        self.run_now
            || self.first_seen.elapsed() >= self.expire_after
            || self.last_updated.elapsed() >= self.save_after
    }
}
