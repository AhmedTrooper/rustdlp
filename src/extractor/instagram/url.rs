use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static IG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?instagram\.com/
        (?:
            p/|
            reel/|
            reels/|
            tv/|
            share/r/|
            share/p/
        )
        (?P<id>[A-Za-z0-9_-]+)
    ",
    )
    .unwrap()
});

pub fn extract_instagram_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = IG_REGEX.captures(trimmed)
        && let Some(id) = captures.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("instagram.com")
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        for (i, seg) in segments.iter().enumerate() {
            if matches!(*seg, "p" | "reel" | "reels" | "tv" | "r")
                && let Some(next) = segments.get(i + 1)
                && !next.is_empty()
            {
                return Ok(VideoId::new(*next));
            }
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_instagram_url(input: &str) -> bool {
    extract_instagram_video_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_instagram_video_id() {
        assert_eq!(
            extract_instagram_video_id("https://www.instagram.com/reel/C3b4Xy0Lq5M/")
                .unwrap()
                .as_str(),
            "C3b4Xy0Lq5M"
        );
        assert_eq!(
            extract_instagram_video_id("https://instagram.com/p/C3b4Xy0Lq5M")
                .unwrap()
                .as_str(),
            "C3b4Xy0Lq5M"
        );
    }
}
