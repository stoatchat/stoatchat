use tempfile::NamedTempFile;

pub fn determine_mime_type(f: &mut NamedTempFile, buf: &[u8], file_name: &str) -> &'static str {
    if file_name.to_lowercase().ends_with(".apk") {
        return "application/vnd.android.package-archive";
    } else if file_name.to_lowercase().ends_with(".exe") {
        return "application/vnd.microsoft.portable-executable";
    }

    let kind = infer::get_from_path(f.path()).expect("file read successfully");
    let mime_type = if let Some(kind) = kind {
        kind.mime_type()
    } else {
        "application/octet-stream"
    };

    if mime_type == "application/octet-stream" && simdutf8::basic::from_utf8(buf).is_ok() {
        if file_name.to_lowercase().ends_with(".svg") {
            return "image/svg+xml";
        } else {
            return "plain/text";
        }
    }

    // Specify the MP4 mime type further by looking at the media streams, as some MP4 files are audio, not video.
    if mime_type == "video/mp4" {
        let Ok(probe) = ffprobe::ffprobe(f.path()) else {
            return mime_type;
        };

        let (mut has_audio, mut has_video) = (false, false);

        for codec_type in probe
            .streams
            .iter()
            .filter_map(|stream| stream.codec_type.as_deref())
        {
            match codec_type {
                "audio" => has_audio = true,
                "video" => has_video = true,
                _ => {}
            }
        }

        if has_audio && !has_video {
            return "audio/mp4";
        }
    }

    mime_type
}
