use iso8601_timestamp::Timestamp;

use crate::File;

auto_derived_partial!(
    /// File hash
    pub struct FileHash {
        /// Sha256 hash of the file
        #[serde(rename = "_id")]
        pub id: String,
        /// Sha256 hash of file after it has been processed
        pub processed_hash: String,

        /// When this file was created in system
        pub created_at: Timestamp,

        /// The bucket this file is stored in
        pub bucket_id: String,
        /// The path at which this file exists in
        pub path: String,
        /// Cryptographic nonce used to encrypt this file
        pub iv: String,

        /// Parsed metadata of this file
        pub metadata: Metadata,
        /// Raw content type of this file
        pub content_type: String,
        /// Size of this file (in bytes)
        pub size: isize,
    },
    "PartialFileHash"
);

auto_derived!(
    /// Metadata associated with a file
    #[serde(tag = "type")]
    #[derive(Default)]
    pub enum Metadata {
        /// File is just a generic uncategorised file
        #[default]
        File,
        /// File contains textual data and should be displayed as such
        Text,
        /// File is an image with specific dimensions
        Image {
            width: isize,
            height: isize,
            #[serde(default)]
            animated: bool,
        },
        /// File is a video with specific dimensions
        Video { width: isize, height: isize },
        /// File is audio
        Audio,
    }
);

impl FileHash {
    /// Create a file from a file hash
    pub fn into_file(
        &self,
        id: String,
        tag: String,
        filename: String,
        uploader_id: String,
    ) -> File {
        File {
            id,
            tag,
            filename,
            hash: Some(self.id.clone()),

            uploaded_at: Some(Timestamp::now_utc()),
            uploader_id: Some(uploader_id),

            used_for: None,

            deleted: None,
            reported: None,

            // TODO: remove this data
            metadata: self.metadata.clone(),
            content_type: self.content_type.clone(),
            size: self.size,

            // TODO: superseded by "used_for"
            message_id: None,
            object_id: None,
            server_id: None,
            user_id: None,
        }
    }
}
