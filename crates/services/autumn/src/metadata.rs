use std::io::Cursor;

use crate::utils::apply_icc_profile;
use image::{GenericImageView, ImageError, ImageReader};
use revolt_database::Metadata;
use revolt_files::{image_size, is_animated, video_size};
use tempfile::NamedTempFile;

/// Intersection of what infer can detect and what image-rs supports
///
/// Note: imagesize crate also supports all of these, so we use that for quick size probing.
static SUPPORTED_IMAGE_MIME: [&str; 9] = [
    "image/avif",
    "image/bmp",
    "image/gif",
    "image/vnd.microsoft.icon",
    "image/jpeg",
    "image/jxl", // not supported by image-rs but we shim it
    "image/png",
    "image/tiff",
    "image/webp",
];

/// Generate metadata from file, using mime type as a hint
pub fn generate_metadata(f: &NamedTempFile, mime_type: &str) -> Metadata {
    if SUPPORTED_IMAGE_MIME.contains(&mime_type) {
        image_size(f)
            .map(|(width, height)| Metadata::Image {
                width: width as isize,
                height: height as isize,
                thumbhash: (|| {
                    let reader = ImageReader::open(f).ok()?.with_guessed_format().ok()?;
                    let mut decoder = reader.into_decoder().ok()?;
                    let icc_profile = image::ImageDecoder::icc_profile(&mut decoder)
                        .ok()
                        .flatten();
                    let mut img = image::DynamicImage::from_decoder(decoder).ok()?;

                    if let Some(icc) = icc_profile {
                        img = apply_icc_profile(img, &icc);
                    }

                    let img = img.thumbnail(100, 100);
                    let (width, height) = img.dimensions();
                    Some(thumbhash::rgba_to_thumb_hash(
                        width as usize,
                        height as usize,
                        &img.into_rgba8().into_raw(),
                    ))
                })(),
                animated: is_animated(f, mime_type).or(Some(false)),
            })
            .unwrap_or_default()
    } else if mime_type.starts_with("video/") {
        video_size(f)
            .map(|(width, height)| Metadata::Video {
                width: width as isize,
                height: height as isize,
            })
            .unwrap_or_default()
    } else if mime_type.starts_with("audio/") {
        Metadata::Audio
    } else if mime_type == "plain/text" {
        Metadata::Text
    } else {
        Metadata::File
    }
}

/// Subroutine to ensure data isn't corrupted
pub fn validate_from_metadata(
    reader: Cursor<Vec<u8>>,
    metadata: Metadata,
    mime_type: &str,
) -> Metadata {
    if let Metadata::Image { .. } = &metadata {
        if mime_type == "image/jxl" {
            // Check if we can read using jxl-oxide crate
            if jxl_oxide::JxlImage::builder()
                .read(reader)
                .inspect_err(|err| tracing::error!("Failed to read JXL! {err:?}"))
                .is_err()
            {
                return Metadata::File;
            }
        } else if matches!(
            // Check if we can read using image-rs crate
            image::ImageReader::new(reader)
                .with_guessed_format()
                .inspect_err(|err| tracing::error!("Failed to read image! {err:?}"))
                .map(|f| f.decode()),
            Err(_) | Ok(Err(_))
        ) {
            return Metadata::File;
        }
    }

    metadata
}
