use std::time::Duration;

use revolt_database::Database;
use revolt_files::delete_from_s3;
use revolt_result::{ErrorType, Result};
use tokio::time::sleep;
use tracing::{info, info_span, warn, Instrument};

use crate::traced_run;

pub async fn task(db: Database, _: revolt_database::AMQP) -> Result<()> {
    loop {
        traced_run("file_deletion", async {
            let files = db.fetch_deleted_attachments().await?;
            info!(count = files.len(), "Fetched deleted files");

            for file in files {
                async {
                    if let Some(hash) = &file.hash {
                        match db.fetch_attachment_hash(hash).await {
                            Ok(file_hash) => {
                                let count = db.count_file_hash_references(&file_hash).await?;

                                // No other files reference this file on disk anymore
                                if count <= 1 {
                                    // Delete from S3
                                    delete_from_s3(&file_hash.bucket_id, &file_hash.path).await?;

                                    // Delete the hash
                                    db.delete_attachment_hash(&file_hash.id).await?;
                                    info!(hash_id = %file_hash.id, "Deleted file hash");
                                }
                            }
                            // Hash is already gone
                            Err(error) if matches!(error.error_type, ErrorType::NotFound) => {
                                warn!(file_id = %file.id, %hash, "File references missing hash");
                            }
                            Err(error) => return Err(error),
                        }
                    }

                    // Delete the file
                    db.delete_attachment(&file.id).await?;
                    info!(file_id = %file.id, "Deleted file");

                    Ok(())
                }
                .instrument(info_span!("file.delete", file_id = %file.id))
                .await?;
            }

            Ok(())
        })
        .await?;

        sleep(Duration::from_secs(60)).await;
    }
}
