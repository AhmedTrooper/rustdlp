use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static BILIBILI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?bilibili\.com/
        (?:
            video/|
            festival/[^/?\#]+\?bvid=
        )
        (?P<id>BV[0-9A-Za-z]+|av\d+)
    ",
    )
    .unwrap()
});

pub fn extract_bilibili_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(cap) = BILIBILI_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("bilibili.com")
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        if let Some(pos) = segments.iter().position(|&s| s == "video")
            && let Some(id) = segments.get(pos + 1)
        {
            return Ok(VideoId::new(*id));
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_bilibili_url(input: &str) -> bool {
    extract_bilibili_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bilibili_id() {
        assert_eq!(
            extract_bilibili_id("https://www.bilibili.com/video/BV1xx411c7mD")
                .unwrap()
                .as_str(),
            "BV1xx411c7mD"
        );
    }
}
