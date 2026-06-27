use std::time::Duration;

use log::warn;
use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        let success = db.remove_dangling_members().await;
        if let Err(s) = success {
            revolt_config::capture_error(&s);
            warn!("Failed to prune dangling members: {:?}", &s);
        }

        sleep(Duration::from_secs(90)).await;
    }
}
