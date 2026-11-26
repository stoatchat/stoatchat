use std::time::Duration;

use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;
use tracing::info;

use crate::traced_run;

pub async fn task(db: Database) -> Result<()> {
    loop {
        traced_run("prune_mfa_tickets", async {
            let count = db.delete_expired_tickets().await?;
            info!(count, "Pruned expired MFA tickets");

            Ok(())
        })
        .await?;

        sleep(Duration::from_mins(5)).await
    }
}
