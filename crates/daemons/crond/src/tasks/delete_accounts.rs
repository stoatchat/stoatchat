use std::time::Duration;

use revolt_database::{Database, AMQP};
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database, amqp: AMQP) -> Result<()> {
    loop {
        let accounts = db.fetch_accounts_due_for_deletion().await?;
        let count = accounts.len();

        for mut account in accounts {
            let mut user = db.fetch_user(&account.id).await?;

            user.delete(&db, Some(&amqp)).await?;
            account.mark_deleted(&db).await?;
        }

        log::info!("Deleted {count} accounts.");

        sleep(Duration::from_hours(1)).await
    }
}
