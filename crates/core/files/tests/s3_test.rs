use revolt_files::{EncryptionKey, FileStorageRepository, S3Storage};

#[tokio::test]
async fn test_upload_and_download() {
    let encryption = EncryptionKey::from_config().await;
    let s3 = S3Storage::from_config(encryption).await;

    let buf = [67];
    let bucket_id = uuid::Uuid::new_v4().to_string();

    s3.create_bucket(&bucket_id).await.unwrap();

    let iv = s3
        .encrypt_and_upload_file(&bucket_id, "/my-file", &buf)
        .await
        .unwrap();

    let buf = s3
        .fetch_and_decrypt_file(&bucket_id, "/my-file", &iv)
        .await
        .unwrap();

    assert_eq!(buf.len(), 1);
    assert_eq!(buf[0], 67);
}

#[tokio::test]
async fn test_upload_and_delete() {
    let encryption = EncryptionKey::from_config().await;
    let s3 = S3Storage::from_config(encryption).await;

    let buf = [67];
    let bucket_id = uuid::Uuid::new_v4().to_string();

    s3.create_bucket(&bucket_id).await.unwrap();

    s3.encrypt_and_upload_file(&bucket_id, "/my-file", &buf)
        .await
        .unwrap();

    s3.delete_file(&bucket_id, "/my-file").await.unwrap();
}
