use anyhow::Result;

#[async_trait::async_trait]
pub trait FileStorageRepository: Send + Sync + 'static {
    async fn create_bucket(&self, bucket_id: &str) -> anyhow::Result<()>;

    async fn fetch_and_decrypt_file(
        &self,
        bucket_id: &str,
        path: &str,
        iv: &str,
    ) -> Result<Vec<u8>>;

    async fn encrypt_and_upload_file(
        &self,
        bucket_id: &str,
        path: &str,
        buf: &[u8],
    ) -> Result<String>;

    async fn delete_file(&self, bucket_id: &str, path: &str) -> Result<()>;
}
