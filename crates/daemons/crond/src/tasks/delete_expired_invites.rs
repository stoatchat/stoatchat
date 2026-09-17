use std::time::Duration;

use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        let count = db.delete_expired_invites().await?;

        log::info!("Deleted {count} expired invites.");

        sleep(Duration::from_hours(1)).await
    }
}