use revolt_result::Result;

use crate::FileHash;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractAttachmentHashes: Sync + Send {
    /// Insert a new attachment hash into the database.
    async fn insert_attachment_hash(&self, hash: &FileHash) -> Result<()>;

    /// Fetch an attachment hash entry by sha256 hash.
    async fn fetch_attachment_hash(&self, hash: &str) -> Result<FileHash>;

    /// Update an attachment hash nonce value.
    async fn set_attachment_hash_nonce(&self, hash: &str, nonce: &str) -> Result<()>;

    /// Updates the attachments animated metadata value.
    ///
    /// The primary use for this is to update the metadata for existing uploaded files, this
    /// can only be used for images.
    async fn set_attachment_hash_animated(&self, hash: &str, animated: bool) -> Result<()>;

    /// Delete attachment hash by id.
    async fn delete_attachment_hash(&self, id: &str) -> Result<()>;
}
