use std::io::Cursor;

use revolt_files::{EncryptionKey, FileStorageRepository, MediaImpl, MediaRepository, S3Storage};

#[tokio::test]
async fn test_image_roundtrip_png() {
    let media = MediaImpl::from_config().await;
    let encryption = EncryptionKey::from_config().await;
    let s3 = S3Storage::from_config(encryption).await;

    let buf = include_bytes!("./assets/test.png");
    let bucket_id = uuid::Uuid::new_v4().to_string();

    s3.create_bucket(&bucket_id).await.unwrap();

    let mut reader = Cursor::new(buf);
    media.decode_image(&mut reader, "image/png").unwrap();

    let iv = s3
        .encrypt_and_upload_file(&bucket_id, "/my-file", buf)
        .await
        .unwrap();

    let buf = s3
        .fetch_and_decrypt_file(&bucket_id, "/my-file", &iv)
        .await
        .unwrap();

    let mut reader = Cursor::new(buf);
    media.decode_image(&mut reader, "image/png").unwrap();
}
