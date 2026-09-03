use std::time::Duration;

use log::warn;
use revolt_config::config;
use revolt_database::Database;
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    let delay = config().await.features.advanced.ephemeral_delay;

    loop {
        let success = db.prune_ephemeral(delay).await;
        if let Err(s) = success {
            revolt_config::capture_error(&s);
            warn!("Failed to prune ephemeral messages: {:?}", &s);
        }

        sleep(Duration::from_secs(90)).await;
    }
}
