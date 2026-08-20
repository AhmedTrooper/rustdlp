use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static YOUTUBE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:https?://)?(?:www\.|m\.|music\.)?(?:youtube\.com/(?:watch\?v=|embed/|v/|shorts/|live/)|youtu\.be/)([a-zA-Z0-9_-]{11})").unwrap()
});

static RAW_ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]{11}$").unwrap());

pub fn extract_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if RAW_ID_REGEX.is_match(trimmed) {
        return Ok(VideoId::new(trimmed));
    }

    if let Some(captures) = YOUTUBE_REGEX.captures(trimmed)
        && let Some(id) = captures.get(1)
    {
        return Ok(VideoId::new(id.as_str()));
    }

    // Try parsing as standard URL with query params
    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
    {
        if domain.contains("youtube.com") {
            for (k, v) in parsed.query_pairs() {
                if k == "v" && v.len() == 11 {
                    return Ok(VideoId::new(v.as_ref()));
                }
            }
            // Path based check: /shorts/ID or /embed/ID
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            if segments.len() >= 2
                && matches!(segments[0], "shorts" | "embed" | "v" | "live")
                && segments[1].len() == 11
            {
                return Ok(VideoId::new(segments[1]));
            }
        } else if domain.contains("youtu.be") {
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            if let Some(first) = segments.first()
                && first.len() == 11
            {
                return Ok(VideoId::new(*first));
            }
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn extract_playlist_id(input: &str) -> Option<String> {
    let trimmed = input.trim();

    if trimmed.starts_with("PL")
        || trimmed.starts_with("RD")
        || trimmed.starts_with("UU")
        || trimmed.starts_with("FL")
    {
        return Some(trimmed.to_string());
    }

    if let Ok(parsed) = Url::parse(trimmed) {
        for (k, v) in parsed.query_pairs() {
            if k == "list" && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            extract_video_id("dQw4w9WgXcQ").unwrap().as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ?t=10")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn test_extract_playlist_id() {
        assert_eq!(
            extract_playlist_id(
                "https://www.youtube.com/playlist?list=PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj"
            ),
            Some("PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj".into())
        );
        assert_eq!(
            extract_playlist_id(
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj"
            ),
            Some("PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj".into())
        );
    }
}
