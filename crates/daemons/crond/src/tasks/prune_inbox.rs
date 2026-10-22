use std::time::Duration;
use revolt_database::{Database, Inbox};
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        let count = Inbox::prune(&db).await?;

        log::info!("Deleted {count} inbox items.");

        sleep(Duration::from_hours(1)).await
    }
}
