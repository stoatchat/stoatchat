use std::time::{Duration, SystemTime};

use revolt_config::config;
use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        let retention_days = config().await.features.limits.global.inbox_retention_days;

        if retention_days > 0 {
            let retention = Duration::from_secs(retention_days as u64 * 24 * 60 * 60);

            if let Some(cutoff) = SystemTime::now().checked_sub(retention) {
                match db.prune_inbox_entries(cutoff).await {
                    Ok(0) => {}
                    Ok(count) => log::info!("Deleted {count} inbox items."),
                    Err(e) => {
                        revolt_config::capture_error(&e);
                    }
                }
            }
        }

        sleep(Duration::from_hours(1)).await;
    }
}