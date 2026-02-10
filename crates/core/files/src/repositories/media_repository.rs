use anyhow::Result;
use image::DynamicImage;
use std::io::{BufRead, Read, Seek};
use tempfile::NamedTempFile;
use thiserror::Error;

pub trait MediaRepository: Send + Sync + 'static {
    fn image_size(&self, f: &NamedTempFile) -> Option<(usize, usize)>;
    fn image_size_vec(&self, v: &[u8], mime: &str) -> Option<(usize, usize)>;

    fn decode_image<R: Read + BufRead + Seek>(
        &self,
        reader: &mut R,
        mime: &str,
    ) -> Result<DynamicImage, MediaError>;

    fn is_valid_image<R: Read + BufRead + Seek>(&self, reader: &mut R, mime: &str) -> bool;

    fn create_thumbnail(&self, image: DynamicImage, tag: &str) -> Vec<u8>;

    fn video_size(&self, f: &NamedTempFile) -> Option<(i64, i64)>;
}

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("image processing failed because {cause}")]
    ImageProcessingFailed { cause: String },
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
