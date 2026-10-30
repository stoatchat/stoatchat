//! Retries persisted inbox fan-out jobs left over from interrupted sends.

use std::time::Duration;

use revolt_config::capture_error;
use revolt_result::Result;

use crate::Database;

const RETRY_INTERVAL: Duration = Duration::from_secs(10);
const BATCH_SIZE: i64 = 100;

pub async fn worker(db: Database) {
    loop {
        if let Err(e) = process_pending_jobs(&db).await {
            capture_error(&e);
        }

        tokio::time::sleep(RETRY_INTERVAL).await;
    }
}

pub async fn process_pending_jobs(db: &Database) -> Result<()> {
    for job in db.fetch_inbox_jobs(BATCH_SIZE).await? {
        if let Err(e) = job.process(db).await {
            capture_error(&e);
        }
    }

    Ok(())
}
