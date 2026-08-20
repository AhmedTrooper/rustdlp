use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static TWITTER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?(?:twitter\.com|x\.com|fxtwitter\.com|vxtwitter\.com)/
        (?:
            [^/]+/status/|
            i/status/|
            i/web/status/
        )
        (?P<id>\d+)
    ",
    )
    .unwrap()
});

pub fn extract_twitter_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = TWITTER_REGEX.captures(trimmed)
        && let Some(id) = captures.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && (domain.contains("twitter.com") || domain.contains("x.com"))
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        if let Some(pos) = segments.iter().position(|&s| s == "status")
            && let Some(id) = segments.get(pos + 1)
            && let Ok(_) = id.parse::<u64>()
        {
            return Ok(VideoId::new(*id));
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_twitter_url(input: &str) -> bool {
    extract_twitter_video_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_twitter_video_id() {
        assert_eq!(
            extract_twitter_video_id("https://twitter.com/NASA/status/1812879590806950293")
                .unwrap()
                .as_str(),
            "1812879590806950293"
        );
        assert_eq!(
            extract_twitter_video_id("https://x.com/SpaceX/status/1780000000000000000")
                .unwrap()
                .as_str(),
            "1780000000000000000"
        );
    }
}
