use anyhow::Result;
use image::{DynamicImage, ImageBuffer, ImageDecoder, ImageReader, Limits};
use jxl_oxide::integration::JxlDecoder;
use revolt_config::report_internal_error;
use std::io::{BufRead, Read, Seek};
use tempfile::NamedTempFile;
use tiny_skia::Pixmap;
use usvg::Transform;

use crate::{MediaError, MediaRepository};

/// Maximum width or height of an image we are willing to decode
const MAX_DECODE_PIXEL_SIDE: u32 = 10_000;
/// Maximum total pixel count of an image we are willing to decode
const MAX_DECODE_PIXELS: u64 = 40_000_000;
/// Maximum memory a decoder may allocate (matches image-rs default)
const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;

/// Whether an image of the given dimensions is too large for us to decode
pub fn exceeds_decode_limits(width: u64, height: u64) -> bool {
    width > MAX_DECODE_PIXEL_SIDE as u64
        || height > MAX_DECODE_PIXEL_SIDE as u64
        || width.saturating_mul(height) > MAX_DECODE_PIXELS
}

/// Check whether the image data provided by a reader contains animation data
fn is_animated_reader<R: BufRead + Seek>(reader: R, mime: &str) -> Option<bool> {
    match mime {
        // Current behaviour is to assume GIFs are animated, this checks for at least 2 frames
        "image/gif" => {

            let mut options = gif::DecodeOptions::new();
            options.skip_frame_decoding(true);
            let mut decoder = options.read_info(reader).ok()?;

            if exceeds_decode_limits(decoder.width().into(), decoder.height().into()) {
                return None;
            }

            if decoder.next_frame_info().ok()?.is_none() {
                return Some(false);
            }

            Some(decoder.next_frame_info().ok()?.is_some())
        }
        "image/png" | "image/apng" => {
            let decoder = image::codecs::png::PngDecoder::new(reader).ok()?;
            decoder.is_apng().ok()
        }
        "image/webp" => {
            let decoder = image::codecs::webp::WebPDecoder::new(reader).ok()?;
            Some(decoder.has_animation())
        }
        _ => Some(false),
    }
}

fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODE_PIXEL_SIDE);
    limits.max_image_height = Some(MAX_DECODE_PIXEL_SIDE);
    // Technically this only leaves space for 2 images to be processed at a time on the default 1gb limit but its probably fine
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

fn decode_with_limits(mut decoder: impl ImageDecoder) -> Result<DynamicImage, MediaError> {
    let (width, height) = decoder.dimensions();
    if exceeds_decode_limits(width.into(), height.into()) {
        return Err(MediaError::ImageProcessingFailed {
            cause: format!("image dimensions {width}x{height} exceed decode limits"),
        });
    }

    // Reserve the output buffer so the decoder only gets whats left
    let mut limits = decode_limits();
    limits
        .reserve(decoder.total_bytes())
        .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;
    decoder
        .set_limits(limits)
        .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

    DynamicImage::from_decoder(decoder).map_err(|e| MediaError::from(anyhow::anyhow!(e)))
}

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

    fn is_animated(&self, f: &NamedTempFile, mime: &str) -> Option<bool> {
        let file = std::fs::File::open(f.path()).ok()?;
        is_animated_reader(std::io::BufReader::new(file), mime)
    }

    fn is_animated_vec(&self, v: &[u8], mime: &str) -> Option<bool> {
        is_animated_reader(std::io::Cursor::new(v), mime)
    }

    #[tracing::instrument(skip(self, v))]
    fn image_size_vec(&self, v: &[u8], mime: &str) -> Option<(usize, usize)> {
        match mime {
            "image/svg+xml" => {
                let mut options = usvg::Options::default();
                options.image_href_resolver.resolve_string = Box::new(|_, _| None);
                let tree = report_internal_error!(usvg::Tree::from_data(v, &options)).ok()?;

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

    #[tracing::instrument(skip(self, reader))]
    fn decode_image<R: Read + BufRead + Seek>(
        &self,
        reader: &mut R,
        mime: &str,
    ) -> Result<DynamicImage, MediaError> {
        match mime {
            "image/jxl" => {
                let decoder =
                    JxlDecoder::new(reader).map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                decode_with_limits(decoder)
            }
            "image/svg+xml" => {
                let mut buf = Vec::new();
                reader
                    .read_to_end(&mut buf)
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                let mut options = usvg::Options::default();
                options.image_href_resolver.resolve_string = Box::new(|_, _| None);

                let tree: usvg::Tree = usvg::Tree::from_data(&buf, &options)
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                let size = tree.size();

                // scale down svg to a max of 500x500
                let scale = (500. / size.width()).min(500. / size.height()).min(1.);

                let width = size.width() * scale;
                let height = size.height() * scale;

                let mut pixmap = Pixmap::new(width as u32, height as u32).ok_or_else(|| {
                    MediaError::ImageProcessingFailed {
                        cause: "failed to create Pixmap, likely zero sized".to_string(),
                    }
                })?;

                let mut pixmap_mut = pixmap.as_mut();
                resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap_mut);

                Ok(DynamicImage::ImageRgba8(
                    ImageBuffer::from_vec(width as u32, height as u32, pixmap.data().to_vec())
                        .ok_or_else(|| MediaError::ImageProcessingFailed {
                            cause: "buffer is not big enough".to_string(),
                        })?,
                ))
            }
            _ => {
                let image: ImageReader<&mut R> = image::ImageReader::new(reader)
                    .with_guessed_format()
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                let decoder = image
                    .into_decoder()
                    .map_err(|e| MediaError::from(anyhow::anyhow!(e)))?;

                decode_with_limits(decoder)
            }
        }
    }

    #[tracing::instrument(skip(self, reader))]
    fn is_valid_image<R: Read + BufRead + Seek>(&self, reader: &mut R, mime: &str) -> bool {
        match mime {
            "image/jxl" => jxl_oxide::JxlImage::builder()
                .read(reader)
                .inspect_err(|err| tracing::error!("Failed to read JXL! {err:?}"))
                .is_ok(),
            _ => image::ImageReader::new(reader)
                .with_guessed_format()
                .map_err(|e| MediaError::from(anyhow::anyhow!(e)))
                .and_then(|image| {
                    image
                        .into_decoder()
                        .map_err(|e| MediaError::from(anyhow::anyhow!(e)))
                })
                .and_then(decode_with_limits)
                .inspect_err(|err| tracing::error!("Failed to read image! {err:?}"))
                .is_ok(),
        }
    }

    #[tracing::instrument(skip(self, image))]
    fn create_thumbnail(&self, image: DynamicImage, tag: &str) -> Vec<u8> {
        let [w, h] = self.config.preview.get(tag).unwrap();

        let image = image.thumbnail(image.width().min(*w as u32), image.height().min(*h as u32));
        let image = match image {
            DynamicImage::ImageRgb8(_) => image,
            DynamicImage::ImageRgba8(_) => image,
            _ => {
                if image.has_alpha() {
                    image.to_rgba8().into()
                } else {
                    image.to_rgb8().into()
                }
            }
        };

        let encoder = webp::Encoder::from_image(&image).expect("Could not create encoder.");
        if self.config.webp_quality != 100.0 {
            encoder.encode(self.config.webp_quality).to_vec()
        } else {
            encoder.encode_lossless().to_vec()
        }
    }

    #[tracing::instrument(skip_all)]
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
    use crate::{MediaImpl, MediaRepository};
    use std::io::{Cursor, Write};
    use tempfile::NamedTempFile;

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
    async fn asset_test_jpeg_is_not_animated() {
        let media = MediaImpl::from_config().await;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(include_bytes!("../../tests/assets/test.jpeg"))
            .unwrap();
        assert_eq!(media.is_animated(&f, "image/jpeg"), Some(false));
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
    async fn asset_test_png_is_not_animated() {
        let media = MediaImpl::from_config().await;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(include_bytes!("../../tests/assets/test.png"))
            .unwrap();
        assert_eq!(media.is_animated(&f, "image/png"), Some(false));
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
    async fn asset_test_floating_point_png() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/test-float.png");
        assert_eq!(media.image_size_vec(buf, "image/png"), Some((300, 300)));

        let mut reader = Cursor::new(buf);
        let image = media.decode_image(&mut reader, "image/png").unwrap();
        media.create_thumbnail(image, "avatars");
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
    async fn asset_test_animated_png_is_animated() {
        let media = MediaImpl::from_config().await;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(include_bytes!("../../tests/assets/anim-icos.apng"))
            .unwrap();
        assert_eq!(media.is_animated(&f, "image/png"), Some(true));
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
    async fn asset_test_oversized_jxl_rejected() {
        let media = MediaImpl::from_config().await;
        // dice.jxl truncated to 4 KiB with its size header patched to declare 10922x8192
        let buf = include_bytes!("../../tests/assets/oversized.jxl");

        let mut reader = Cursor::new(buf);
        let err = media.decode_image(&mut reader, "image/jxl").unwrap_err();
        assert!(
            err.to_string().contains("exceed decode limits"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn asset_test_oversized_png_rejected() {
        let media = MediaImpl::from_config().await;
        // 7000x7000 is within the per-side limit but exceeds the megapixel limit
        let buf = include_bytes!("../../tests/assets/oversized.png");

        let mut reader = Cursor::new(buf);
        let err = media.decode_image(&mut reader, "image/png").unwrap_err();
        assert!(
            err.to_string().contains("exceed decode limits"),
            "unexpected error: {err}"
        );
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
    async fn asset_test_webp_is_not_animated() {
        let media = MediaImpl::from_config().await;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(include_bytes!("../../tests/assets/dice.webp"))
            .unwrap();
        assert_eq!(media.is_animated(&f, "image/webp"), Some(false));
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
    async fn asset_test_animated_webp_is_animated() {
        let media = MediaImpl::from_config().await;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(include_bytes!("../../tests/assets/anim-icos.webp"))
            .unwrap();
        assert_eq!(media.is_animated(&f, "image/webp"), Some(true));
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

    #[tokio::test]
    async fn asset_test_animated_gif_is_animated() {
        let media = MediaImpl::from_config().await;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(include_bytes!("../../tests/assets/anim-icos.gif"))
            .unwrap();
        assert_eq!(media.is_animated(&f, "image/gif"), Some(true));
    }

    #[tokio::test]
    async fn asset_test_is_animated_vec() {
        let media = MediaImpl::from_config().await;
        let cases: [(&[u8], &str, bool); 7] = [
            (include_bytes!("../../tests/assets/test.jpeg"), "image/jpeg", false),
            (include_bytes!("../../tests/assets/test.png"), "image/png", false),
            (include_bytes!("../../tests/assets/dice.webp"), "image/webp", false),
            (include_bytes!("../../tests/assets/anim-icos.apng"), "image/png", true),
            (include_bytes!("../../tests/assets/anim-icos.apng"), "image/apng", true),
            (include_bytes!("../../tests/assets/anim-icos.webp"), "image/webp", true),
            (include_bytes!("../../tests/assets/anim-icos.gif"), "image/gif", true),
        ];

        for (buf, mime, animated) in cases {
            assert_eq!(media.is_animated_vec(buf, mime), Some(animated), "{mime}");
            assert_eq!(crate::is_animated_vec(buf, mime), Some(animated), "{mime}");
        }
    }

    #[tokio::test]
    async fn asset_test_truncated_gif_is_animated() {
        let media = MediaImpl::from_config().await;
        let buf = include_bytes!("../../tests/assets/anim-icos.gif");

        // The second frame's image data spans bytes 8830..16772, so cutting the file off
        // partway through it leaves one good frame followed by one that fails to decode
        let truncated = &buf[..12_000];
        assert_eq!(media.is_animated_vec(truncated, "image/gif"), Some(true));

        // Cutting it off during the first frame's image data (808..8822) leaves nothing usable
        let truncated = &buf[..4_000];
        assert_eq!(media.is_animated_vec(truncated, "image/gif"), None);
    }

    #[tokio::test]
    async fn asset_test_oversized_gif_is_not_decoded() {
        let media = MediaImpl::from_config().await;
        // Header-only GIF declaring a 20000x20000 logical screen
        let mut buf = b"GIF89a".to_vec();
        buf.extend_from_slice(&20000u16.to_le_bytes());
        buf.extend_from_slice(&20000u16.to_le_bytes());
        buf.extend_from_slice(&[0, 0, 0, 0x3B]);

        assert_eq!(media.is_animated_vec(&buf, "image/gif"), None);
    }

    #[tokio::test]
    async fn asset_test_large_canvas_gif_is_animated() {
        let media = MediaImpl::from_config().await;

        // A 9000x4000 logical screen (within decode limits) holding tiny 1x1 frames, decoding
        // these frames as full canvases would allocate ~144 MiB each
        let gif = |frames: usize| {
            let mut buf = b"GIF89a".to_vec();
            buf.extend_from_slice(&9000u16.to_le_bytes());
            buf.extend_from_slice(&4000u16.to_le_bytes());
            // Global colour table of 2 entries, background colour, aspect ratio, palette
            buf.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF]);
            for _ in 0..frames {
                // Image descriptor for a 1x1 frame at 0,0 followed by its LZW data
                buf.extend_from_slice(&[0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0]);
                buf.extend_from_slice(&[0x02, 0x02, 0x44, 0x01, 0x00]);
            }
            buf.push(0x3B);
            buf
        };

        assert_eq!(media.is_animated_vec(&gif(1), "image/gif"), Some(false));
        assert_eq!(media.is_animated_vec(&gif(2), "image/gif"), Some(true));
    }
}
