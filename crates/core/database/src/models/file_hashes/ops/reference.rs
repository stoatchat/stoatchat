use revolt_result::Result;

use crate::{FileHash, Metadata, ReferenceDb};

use super::AbstractAttachmentHashes;

#[async_trait]
impl AbstractAttachmentHashes for ReferenceDb {
    /// Insert a new attachment hash into the database.
    async fn insert_attachment_hash(&self, hash: &FileHash) -> Result<()> {
        let mut hashes = self.file_hashes.lock().await;
        if hashes.contains_key(&hash.id) {
            Err(create_database_error!("insert", "attachment"))
        } else {
            hashes.insert(hash.id.to_string(), hash.clone());
            Ok(())
        }
    }

    /// Fetch an attachment hash entry by sha256 hash.
    async fn fetch_attachment_hash(&self, hash_value: &str) -> Result<FileHash> {
        let hashes = self.file_hashes.lock().await;
        hashes
            .values()
            .find(|&hash| hash.id == hash_value || hash.processed_hash == hash_value)
            .cloned()
            .ok_or(create_error!(NotFound))
    }

    /// Update an attachment hash nonce value.
    async fn set_attachment_hash_nonce(&self, hash: &str, nonce: &str) -> Result<()> {
        let mut hashes = self.file_hashes.lock().await;
        if let Some(file) = hashes.get_mut(hash) {
            file.iv = nonce.to_owned();
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Updates the attachments animated metadata value.
    ///
    /// The primary use for this is to update the metadata for existing uploaded files, this
    /// can only be used for images.
    async fn set_attachment_hash_animated(&self, hash: &str, animated: bool) -> Result<()> {
        let mut hashes = self.file_hashes.lock().await;
        if let Some(FileHash {
            metadata:
                Metadata::Image {
                    animated: Some(animated_metadata),
                    ..
                },
            ..
        }) = hashes.get_mut(hash)
        {
            *animated_metadata = animated;

            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Delete attachment hash by id.
    async fn delete_attachment_hash(&self, id: &str) -> Result<()> {
        let mut file_hashes = self.file_hashes.lock().await;
        if file_hashes.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
