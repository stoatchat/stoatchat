use std::io::Write;

use anyhow::Context;
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client, Config,
};
use revolt_config::FilesS3;

use crate::{EncryptionRepository, FileStorageRepository};

pub struct S3Storage<ER: EncryptionRepository> {
    client: Client,
    encryption: ER,
}

impl<ER: EncryptionRepository> S3Storage<ER> {
    pub async fn from_config(encryption: ER) -> S3Storage<ER> {
        S3Storage::new(encryption, revolt_config::config().await.files.s3)
    }

    pub fn new(encryption: ER, s3_config: FilesS3) -> S3Storage<ER> {
        let provider_name = "my-creds";
        let creds = Credentials::new(
            s3_config.access_key_id,
            s3_config.secret_access_key,
            None,
            None,
            provider_name,
        );

        let config = Config::builder()
            .region(Region::new(s3_config.region))
            .endpoint_url(s3_config.endpoint)
            .force_path_style(s3_config.path_style_buckets)
            .credentials_provider(creds)
            .build();

        S3Storage {
            client: Client::from_conf(config),
            encryption,
        }
    }
}

#[async_trait::async_trait]
impl<ER: EncryptionRepository> FileStorageRepository for S3Storage<ER> {
    async fn create_bucket(&self, bucket_id: &str) -> anyhow::Result<()> {
        self.client
            .create_bucket()
            .bucket(bucket_id)
            .send()
            .await
            .with_context(|| format!("failed to create bucket {bucket_id}"))?;

        Ok(())
    }

    async fn fetch_and_decrypt_file(
        &self,
        bucket_id: &str,
        path: &str,
        iv: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let mut object = self
            .client
            .get_object()
            .bucket(bucket_id)
            .key(path)
            .send()
            .await
            .with_context(|| format!("failed to get object at {path} in {bucket_id}"))?;

        let mut buf = vec![];
        while let Some(bytes) = object.body.next().await {
            let data = bytes?;
            buf.write_all(&data)?;
        }

        if iv.is_empty() {
            Ok(buf)
        } else {
            self.encryption.decrypt_buffer(buf, iv)
        }
    }

    async fn encrypt_and_upload_file(
        &self,
        bucket_id: &str,
        path: &str,
        buf: &[u8],
    ) -> anyhow::Result<String> {
        let (buf, iv) = self.encryption.encrypt_buffer(buf)?;

        self.client
            .put_object()
            .bucket(bucket_id)
            .key(path)
            .body(buf.into())
            .send()
            .await
            .with_context(|| format!("failed to put object at {path} in {bucket_id}"))?;

        Ok(iv)
    }

    async fn delete_file(&self, bucket_id: &str, path: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(bucket_id)
            .key(path)
            .send()
            .await
            .with_context(|| format!("failed to delete object at {path} in {bucket_id}"))?;

        Ok(())
    }
}
