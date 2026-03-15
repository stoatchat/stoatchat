use std::io::{Cursor, Read};

use exif::Reader;
use image::{ImageFormat, ImageReader};
use revolt_config::report_internal_error;
use revolt_database::Metadata;
use revolt_result::{create_error, Result};
use tempfile::NamedTempFile;
use tokio::process::Command;

/// Strip EXIF data from given file and produce new file and metadata
pub async fn strip_metadata(
    file: NamedTempFile,
    buf: Vec<u8>,
    metadata: Metadata,
    mime: &str,
) -> Result<(Vec<u8>, Metadata)> {
    match &metadata {
        Metadata::Image {
            width,
            height,
            animated,
        } => match mime {
            // // little_exif does not appear to parse JPEGs correctly? had 2/2 files fail
            // "image/jpeg" | "image/png" => {
            //     // use little_exif to strip metadata except for orientation and colour profile
            //     // PNGs must also be re-encoded to mitigate CVE-2023-21036
            //     let metadata = revolt_little_exif::metadata::Metadata::new_from_path_with_filetype(
            //         file.path(),
            //         match mime {
            //             "image/jpeg" => revolt_little_exif::filetype::FileExtension::JPEG,
            //             "image/png" => revolt_little_exif::filetype::FileExtension::PNG {
            //                 as_zTXt_chunk: true,
            //             },
            //             _ => unreachable!(),
            //         },
            //     )
            //     .unwrap();
            //     dbg!(metadata.data());
            //     todo!()
            // }
            // Apply orientation manually & strip all other EXIF data
            "image/jpeg" | "image/png" | "image/avif" | "image/tiff" => {
                // Create a reader
                let mut cursor = Cursor::new(buf);

                // Decode the image
                let image = report_internal_error!(report_internal_error!(ImageReader::new(
                    &mut cursor
                )
                .with_guessed_format())?
                .decode());

                // Reset read position
                cursor.set_position(0);

                // Extract orientation data
                let exif_reader = Reader::new();
                let rotation = match exif_reader.read_from_container(&mut cursor) {
                    Ok(exif) => match exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
                        Some(orientation) => orientation.value.get_uint(0).unwrap_or_default(),
                        _ => 0,
                    },
                    _ => 0,
                };

                // Create a buffer to write to
                let mut bytes: Vec<u8> = Vec::new();
                let mut writer = Cursor::new(&mut bytes);

                // Apply the EXIF rotation
                // See https://jdhao.github.io/2019/07/31/image_rotation_exif_info/
                report_internal_error!(match &rotation {
                    2 => image?.fliph(),
                    3 => image?.rotate180(),
                    4 => image?.rotate180().fliph(),
                    5 => image?.rotate90().fliph(),
                    6 => image?.rotate90(),
                    7 => image?.rotate270().fliph(),
                    8 => image?.rotate270(),
                    _ => image?,
                }
                .write_to(
                    &mut writer,
                    match mime {
                        "image/jpeg" => ImageFormat::Jpeg,
                        "image/png" => ImageFormat::Png,
                        "image/avif" => ImageFormat::Avif,
                        "image/tiff" => ImageFormat::Tiff,
                        _ => todo!(),
                    },
                ))?;

                // Calculate dimensions after rotation.
                let (width, height) = match &rotation {
                    2 | 4 | 5 | 7 => (*height, *width),
                    _ => (*width, *height),
                };

                Ok((
                    bytes,
                    Metadata::Image {
                        width,
                        height,
                        animated: *animated,
                    },
                ))
            }
            // JXLs store EXIF data but we don't have the ability to write them
            "image/jxl" => Ok((buf, metadata)),
            // All other images that cannot store EXIF data
            _ => Ok((buf, metadata)),
        },
        // Use ffmpeg to copy video stream and probe new metadata
        Metadata::Video { .. } => match mime {
            // Strip EXIF data by copying video stream
            "video/mp4" | "video/webm" | "video/quicktime" => {
                // Pick the correct file format for ffmpeg
                let ext = match mime {
                    "video/mp4" => "mp4",
                    "video/webm" => "webm",
                    "video/quicktime" => "mov",
                    _ => unreachable!(),
                };

                // Temporary output file
                let mut out_file = report_internal_error!(NamedTempFile::new())?;

                // Process the file with ffmpeg
                report_internal_error!(
                    Command::new("ffmpeg")
                        .args([
                            // Overwrite the temporary file
                            "-y",
                            // Read original uploaded file
                            "-i",
                            file.path().to_str().ok_or(create_error!(InternalError))?,
                            // Strip any metadata
                            "-map_metadata",
                            "-1",
                            // Copy video / audio data to new file
                            "-c:v",
                            "copy",
                            "-c:a",
                            "copy",
                            // Select correct file format
                            "-f",
                            ext,
                            // Save to new temporary file
                            out_file
                                .path()
                                .to_str()
                                .ok_or(create_error!(InternalError))?,
                        ])
                        .output()
                        .await
                )?;

                // Probe the file again
                let metadata = crate::metadata::generate_metadata(&out_file, mime);

                // Read the file from disk
                let mut buf = Vec::<u8>::new();
                report_internal_error!(out_file.read_to_end(&mut buf))?;

                Ok((buf, metadata))
            }
            // Assume all other video formats cannot store EXIF data
            _ => Ok((buf, metadata)),
        },
        // all other file types don't store EXIF data
        _ => Ok((buf, metadata)),
    }
}
