use std::time::Duration;

use revolt_database::{iso8601_timestamp::Timestamp, Database};
use revolt_result::Result;
use tokio::time::sleep;

use log::info;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        // This could just be a single database query
        // ... but timestamps are inconsistently serialised
        // ... sometimes they are dates/numbers, hard to query
        // ... in the future, we could use Postgres instead! :D
        // ...
        // ... on the plus side, it's still only 2 queries

        let files = db.fetch_dangling_files().await?;
        let file_ids: Vec<String> = files
            .into_iter()
            .filter(|file| {
                file.uploaded_at.is_some_and(|uploaded_at| {
                    Timestamp::now_utc().duration_since(uploaded_at) > Duration::from_secs(60 * 60)
                })
            })
            .map(|file| file.id)
            .collect();

        if !file_ids.is_empty() {
            db.mark_attachments_as_deleted(&file_ids).await?;
            info!("Marked {} dangling files for deletion", file_ids.len());
        }

        sleep(Duration::from_secs(60)).await;
    }
}
