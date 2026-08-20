use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static DM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?(?:dailymotion\.com|dai\.ly)/
        (?:
            video/|
            embed/video/|
        )?
        (?P<id>[a-zA-Z0-9]+)
    ",
    )
    .unwrap()
});

pub fn extract_dailymotion_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = DM_REGEX.captures(trimmed)
        && let Some(id) = captures.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && (domain.contains("dailymotion.com") || domain.contains("dai.ly"))
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

pub fn is_dailymotion_url(input: &str) -> bool {
    extract_dailymotion_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dailymotion_id() {
        assert_eq!(
            extract_dailymotion_id("https://www.dailymotion.com/video/x8jzy9c")
                .unwrap()
                .as_str(),
            "x8jzy9c"
        );
        assert_eq!(
            extract_dailymotion_id("https://dai.ly/x8jzy9c")
                .unwrap()
                .as_str(),
            "x8jzy9c"
        );
    }
}
