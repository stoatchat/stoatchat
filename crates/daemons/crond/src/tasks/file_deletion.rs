use std::time::Duration;

use log::info;
use revolt_database::Database;
use revolt_files::delete_from_s3;
use revolt_result::Result;
use tokio::time::sleep;

pub async fn task(db: Database) -> Result<()> {
    loop {
        let files = db.fetch_deleted_attachments().await?;

        for file in files {
            if let Some(hash) = &file.hash {
                let count = db
                    .count_file_hash_references(hash)
                    .await?;

                // No other files reference this file on disk anymore
                if count <= 1 {
                    let file_hash = db
                        .fetch_attachment_hash(hash)
                        .await?;

                    // Delete from S3
                    delete_from_s3(&file_hash.bucket_id, &file_hash.path).await?;

                    // Delete the hash
                    db.delete_attachment_hash(&file_hash.id).await?;
                    info!("Deleted file hash {}", file_hash.id);
                }
            }

            // Delete the file
            db.delete_attachment(&file.id).await?;
            info!("Deleted file {}", file.id);
        }

        sleep(Duration::from_secs(60)).await;
    }
}
