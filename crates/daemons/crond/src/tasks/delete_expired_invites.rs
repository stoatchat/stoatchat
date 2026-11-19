use std::time::Duration;

use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;
use tracing::info;

use crate::traced_run;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        traced_run("delete_expired_invites", async {
            let count = db.delete_expired_invites().await?;

            info!(count, "Deleted expired invites");

            Ok(())
        })
        .await?;

        sleep(Duration::from_hours(1)).await
    }
}