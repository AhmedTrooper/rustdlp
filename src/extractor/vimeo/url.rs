use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static VIMEO_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?vimeo\.com/
        (?:
            video/|
            channels/(?:[^/]+)/|
            groups/(?:[^/]+)/videos/|
            album/(?:[^/]+)/video/|
        )?
        (?P<id>\d+)
    ",
    )
    .unwrap()
});

pub fn extract_vimeo_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = VIMEO_REGEX.captures(trimmed)
        && let Some(id) = captures.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("vimeo.com")
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        if let Some(last) = segments.last()
            && let Ok(_) = last.parse::<u64>()
        {
            return Ok(VideoId::new(*last));
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_vimeo_url(input: &str) -> bool {
    extract_vimeo_video_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vimeo_video_id() {
        assert_eq!(
            extract_vimeo_video_id("https://vimeo.com/76979871")
                .unwrap()
                .as_str(),
            "76979871"
        );
        assert_eq!(
            extract_vimeo_video_id("https://player.vimeo.com/video/76979871")
                .unwrap()
                .as_str(),
            "76979871"
        );
    }
}
