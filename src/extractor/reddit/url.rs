use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static REDDIT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?reddit\.com/
        (?:
            r/(?P<subreddit>[^/]+)/comments/|
            comments/|
            user/[^/]+/comments/
        )
        (?P<id>[A-Za-z0-9]+)
    ",
    )
    .unwrap()
});

static V_REDDIT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:https?://)?v\.redd\.it/(?P<id>[A-Za-z0-9]+)").unwrap());

pub fn extract_reddit_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(cap) = V_REDDIT_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Some(cap) = REDDIT_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && (domain.contains("reddit.com") || domain.contains("redd.it"))
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        for (i, seg) in segments.iter().enumerate() {
            if *seg == "comments"
                && let Some(next) = segments.get(i + 1)
                && !next.is_empty()
            {
                return Ok(VideoId::new(*next));
            }
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_reddit_url(input: &str) -> bool {
    extract_reddit_video_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_reddit_video_id() {
        assert_eq!(
            extract_reddit_video_id(
                "https://www.reddit.com/r/aww/comments/90bu6w/heat_index_was_110_degrees/"
            )
            .unwrap()
            .as_str(),
            "90bu6w"
        );
        assert_eq!(
            extract_reddit_video_id("https://v.redd.it/gyh95hiqc0b11")
                .unwrap()
                .as_str(),
            "gyh95hiqc0b11"
        );
    }
}
