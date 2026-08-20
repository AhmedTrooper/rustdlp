use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static TWITCH_CLIP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:
            clips\.twitch\.tv/(?:embed\?.*?\bclip=|(?:[^/]+/)*)|
            (?:[\w-]+\.)?twitch\.tv/(?:[^/]+/)?clip/
        )
        (?P<id>[^/?\#&]+)
    ",
    )
    .unwrap()
});

static TWITCH_VOD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?twitch\.tv/videos/(?P<id>\d+)
    ",
    )
    .unwrap()
});

pub fn extract_twitch_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(cap) = TWITCH_CLIP_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Some(cap) = TWITCH_VOD_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("twitch.tv")
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        if let Some(last) = segments.into_iter().rev().find(|s| !s.is_empty()) {
            return Ok(VideoId::new(last));
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_twitch_url(input: &str) -> bool {
    extract_twitch_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_twitch_id() {
        assert_eq!(
            extract_twitch_id("https://clips.twitch.tv/FaintLightGullWholeWheat")
                .unwrap()
                .as_str(),
            "FaintLightGullWholeWheat"
        );
        assert_eq!(
            extract_twitch_id("https://www.twitch.tv/videos/123456789")
                .unwrap()
                .as_str(),
            "123456789"
        );
    }
}
