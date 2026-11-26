use std::time::Duration;

use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;
use tracing::error;

use crate::traced_run;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        // errors are logged rather than returned, so a failed prune doesn't restart the task
        let result = traced_run("prune_members", db.remove_dangling_members()).await;
        if let Err(error) = result {
            error!(?error, "Failed to prune dangling members");
        }

        sleep(Duration::from_secs(90)).await;
    }
}
