use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static PINTEREST_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?pinterest\.(?:com|[a-z]{2}(?:\.[a-z]{2})?)/
        (?:
            pin/|
            video/
        )
        (?P<id>\d+)
    ",
    )
    .unwrap()
});

pub fn extract_pinterest_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(cap) = PINTEREST_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("pinterest.")
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        if let Some(last) = segments
            .into_iter()
            .rev()
            .find(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        {
            return Ok(VideoId::new(last));
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_pinterest_url(input: &str) -> bool {
    extract_pinterest_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pinterest_id() {
        assert_eq!(
            extract_pinterest_id("https://www.pinterest.com/pin/123456789012345678/")
                .unwrap()
                .as_str(),
            "123456789012345678"
        );
    }
}
