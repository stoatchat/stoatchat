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

    mime_type
}
