use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static TIKTOK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?tiktok\.com/
        (?:
            @[\w\.-]+/video/|
            embed/|
            v/|
            t/|
            share/video/|
        )
        (?P<id>[A-Za-z0-9_-]+)
    ",
    )
    .unwrap()
});

pub fn extract_tiktok_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = TIKTOK_REGEX.captures(trimmed)
        && let Some(id) = captures.name("id")
        && !id.as_str().is_empty()
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && (domain.contains("tiktok.com") || domain.contains("douyin.com"))
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

pub fn is_tiktok_url(input: &str) -> bool {
    extract_tiktok_video_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tiktok_video_id() {
        assert_eq!(
            extract_tiktok_video_id("https://www.tiktok.com/@tiktok/video/7106594312292453678")
                .unwrap()
                .as_str(),
            "7106594312292453678"
        );
        assert_eq!(
            extract_tiktok_video_id("https://vm.tiktok.com/ZM8wXj8qL/")
                .unwrap()
                .as_str(),
            "ZM8wXj8qL"
        );
    }
}
