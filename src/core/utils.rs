use std::path::{Path, PathBuf};

pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_duration(seconds: u64) -> String {
    let hrs = seconds / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hrs > 0 {
        format!("{:02}:{:02}:{:02}", hrs, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

pub fn format_bitrate(bps: u64) -> String {
    if bps >= 1_000_000 {
        format!("{:.1}M", bps as f64 / 1_000_000.0)
    } else if bps >= 1000 {
        format!("{}k", bps / 1000)
    } else {
        format!("{}bps", bps)
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let forbidden = [
        '/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0', '\n', '\r', '\t',
    ];
    let sanitized: String = name
        .chars()
        .map(|c| if forbidden.contains(&c) { '_' } else { c })
        .collect();

    let trimmed = sanitized.trim_matches(|c: char| c.is_whitespace() || c == '.');
    if trimmed.is_empty() {
        "video".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn resolve_output_path(
    template: &str,
    title: &str,
    video_id: &str,
    uploader: &str,
    ext: &str,
) -> PathBuf {
    let sanitized_title = sanitize_filename(title);
    let sanitized_uploader = sanitize_filename(uploader);

    let mut result = template.to_string();
    if result.contains("%(title)s") {
        result = result.replace("%(title)s", &sanitized_title);
    }
    if result.contains("%(id)s") {
        result = result.replace("%(id)s", video_id);
    }
    if result.contains("%(uploader)s") {
        result = result.replace("%(uploader)s", &sanitized_uploader);
    }
    if result.contains("%(ext)s") {
        result = result.replace("%(ext)s", ext);
    }

    if !result.ends_with(&format!(".{}", ext)) && !template.contains("%(ext)s") {
        // If template doesn't specify ext or end with it
        let path = Path::new(&result);
        if path.extension().is_none() {
            result = format!("{}.{}", result, ext);
        }
    }

    PathBuf::from(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.50 GiB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(213), "03:33");
        assert_eq!(format_duration(3665), "01:01:05");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("Rick Astley - Never Gonna Give You Up (Official Music Video)"),
            "Rick Astley - Never Gonna Give You Up (Official Music Video)"
        );
        assert_eq!(
            sanitize_filename("Test / Video: Title? *"),
            "Test _ Video_ Title_ _"
        );
    }

    #[test]
    fn test_resolve_output_path() {
        let path = resolve_output_path(
            "%(title)s [%(id)s].%(ext)s",
            "Song Title",
            "dQw4w9WgXcQ",
            "Artist",
            "mp4",
        );
        assert_eq!(path, PathBuf::from("Song Title [dQw4w9WgXcQ].mp4"));
    }
}
