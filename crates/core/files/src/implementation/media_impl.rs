use anyhow::Result;
use image::{DynamicImage, ImageBuffer, ImageReader};
use jxl_oxide::integration::JxlDecoder;
use revolt_config::report_internal_error;
use std::io::{BufRead, Read, Seek};
use tempfile::NamedTempFile;
use tiny_skia::Pixmap;

use crate::{MediaError, MediaRepository};

pub struct MediaImpl {
    config: revolt_config::Files,
}

impl MediaImpl {
    pub async fn from_config() -> MediaImpl {
        MediaImpl {
            config: revolt_config::config().await.files,
        }
    }

    pub fn new(config: revolt_config::Files) -> MediaImpl {
        MediaImpl { config }
    }
}

impl MediaRepository for MediaImpl {
    fn image_size(&self, f: &NamedTempFile) -> Option<(usize, usize)> {
        if let Ok(size) = imagesize::size(f.path())
            .inspect_err(|err| tracing::error!("Failed to generate image size! {err:?}"))
        {
            Some((size.width, size.height))
        } else {
            None
        }
    }

    fn image_size_vec(&self, v: &[u8], mime: &str) -> Option<(usize, usize)> {
        match mime {
            "image/svg+xml" => {
                let tree =
                    report_internal_error!(usvg::Tree::from_data(v, &Default::default())).ok()?;

                let size = tree.size();
                Some((size.width() as usize, size.height() as usize))
            }
            _ => {
                if let Ok(size) = imagesize::blob_size(v)
                    .inspect_err(|err| tracing::error!("Failed to generate image size! {err:?}"))
                {
                    Some((size.width, size.height))
                } else {
                    None
                }
            }
        }
    }

    fn decode_image<R: Read + BufRead + Seek>(
        &self,
        reader: &mut R,
        mime: &str,
    ) -> Result<DynamicImage, MediaError> {
        match mime {
            "image/jxl" => {
                let decoder =
                    JxlDecoder::new(reader).map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                DynamicImage::from_decoder(decoder)
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))
            }
            "image/svg+xml" => {
                let mut buf = Vec::new();
                reader
                    .read_to_end(&mut buf)
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                let tree: usvg::Tree = usvg::Tree::from_data(&buf, &Default::default())
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                let size = tree.size();
                let mut pixmap = Pixmap::new(size.width() as u32, size.height() as u32)
                    .ok_or_else(|| MediaError::ImageProcessingFailed {
                        cause: "failed to create Pixmap, likely zero sized".to_string(),
                    })?;

                let mut pixmap_mut = pixmap.as_mut();
                resvg::render(&tree, Default::default(), &mut pixmap_mut);

                Ok(DynamicImage::ImageRgba8(
                    ImageBuffer::from_vec(
                        size.width() as u32,
                        size.height() as u32,
                        pixmap.data().to_vec(),
                    )
                    .ok_or_else(|| MediaError::ImageProcessingFailed {
                        cause: "buffer is not big enough".to_string(),
                    })?,
                ))
            }
            _ => {
                let image: ImageReader<&mut R> = image::ImageReader::new(reader)
                    .with_guessed_format()
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                let image: Result<DynamicImage, MediaError> = image
                    .decode()
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)));

                image
            }
        }
    }

    fn is_valid_image<R: Read + BufRead + Seek>(&self, reader: &mut R, mime: &str) -> bool {
        match mime {
            "image/jxl" => jxl_oxide::JxlImage::builder()
                .read(reader)
                .inspect_err(|err| tracing::error!("Failed to read JXL! {err:?}"))
                .is_ok(),
            _ => !matches!(
                image::ImageReader::new(reader)
                    .with_guessed_format()
                    .inspect_err(|err| tracing::error!("Failed to read image! {err:?}"))
                    .map(|f| f.decode()),
                Err(_) | Ok(Err(_))
            ),
        }
    }

    fn create_thumbnail(&self, image: DynamicImage, tag: &str) -> Vec<u8> {
        let [w, h] = self.config.preview.get(tag).unwrap();

        let image = image.thumbnail(image.width().min(*w as u32), image.height().min(*h as u32));

        let encoder = webp::Encoder::from_image(&image).expect("Could not create encoder.");
        if self.config.webp_quality != 100.0 {
            encoder.encode(self.config.webp_quality).to_vec()
        } else {
            encoder.encode_lossless().to_vec()
        }
    }

    fn video_size(&self, f: &NamedTempFile) -> Option<(i64, i64)> {
        if let Ok(data) = ffprobe::ffprobe(f.path())
            .inspect_err(|err| tracing::error!("Failed to ffprobe file! {err:?}"))
        {
            for stream in data.streams {
                if let (Some(w), Some(h)) = (stream.width, stream.height) {
                    return Some((w, h));
                }
            }

            None
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{MediaImpl, MediaRepository};

    #[tokio::test]
    async fn asset_test_jpeg() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/test.jpeg");
        assert_eq!(media.image_size_vec(buf, "image/jpeg"), Some((655, 582)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/jpeg").unwrap();
        media.create_thumbnail(image, "attachments");
    }

    #[tokio::test]
    async fn asset_test_jpeg_extra_bytes() {
        let media = MediaImpl::from_config().await;
        let buf = [
            &include_bytes!("../../tests/assets/test.jpeg")[..],
            &[0u8; 16],
        ]
        .concat();
        assert_eq!(media.image_size_vec(&buf, "image/jpeg"), Some((655, 582)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/jpeg").unwrap();
        media.create_thumbnail(image, "emojis");
    }

    #[tokio::test]
    async fn asset_test_png() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/test.png");
        assert_eq!(media.image_size_vec(buf, "image/png"), Some((900, 900)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/png").unwrap();
        media.create_thumbnail(image, "emojis");
    }

    #[tokio::test]
    async fn asset_test_png_extra_bytes() {
        let media = MediaImpl::from_config().await;
        let buf = [
            &include_bytes!("../../tests/assets/test.png")[..],
            &[0u8; 16],
        ]
        .concat();
        assert_eq!(media.image_size_vec(&buf, "image/png"), Some((900, 900)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/png").unwrap();
        media.create_thumbnail(image, "emojis");
    }

    #[tokio::test]
    async fn asset_test_corrupted_png() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/corrupted.png");
        assert_eq!(media.image_size_vec(buf, "image/png"), Some((900, 900)));

        let mut reader = Cursor::new(buf);
        media.decode_image(&mut reader, "image/png").unwrap_err();
    }

    #[tokio::test]
    async fn asset_test_animated_png() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/anim-icos.apng");
        assert_eq!(media.image_size_vec(buf, "image/png"), Some((128, 128)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/png").unwrap();
        media.create_thumbnail(image, "attachments");
    }

    #[tokio::test]
    async fn asset_test_jxl() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/dice.jxl");
        assert_eq!(media.image_size_vec(buf, "image/jxl"), Some((800, 600)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/jxl").unwrap();
        media.create_thumbnail(image, "attachments");
    }

    #[tokio::test]
    async fn asset_test_animated_jxl() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/anim-icos.jxl");
        assert_eq!(media.image_size_vec(buf, "image/jxl"), Some((128, 128)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/jxl").unwrap();
        media.create_thumbnail(image, "attachments");
    }

    #[tokio::test]
    async fn asset_test_webp() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/dice.webp");
        assert_eq!(media.image_size_vec(buf, "image/webp"), Some((800, 600)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/webp").unwrap();
        media.create_thumbnail(image, "attachments");
    }

    #[tokio::test]
    async fn asset_test_animated_webp() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/anim-icos.webp");
        assert_eq!(media.image_size_vec(buf, "image/webp"), Some((128, 128)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/webp").unwrap();
        media.create_thumbnail(image, "attachments");
    }

    #[tokio::test]
    async fn asset_test_animated_gif() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/anim-icos.gif");
        assert_eq!(media.image_size_vec(buf, "image/gif"), Some((128, 128)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/gif").unwrap();
        media.create_thumbnail(image, "attachments");
    }
}
