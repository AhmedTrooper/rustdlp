use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static LINKEDIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?linkedin\.com/
        (?:
            posts/[^/?\#]+-(?P<post_id>\d+)-\w{4}|
            feed/update/urn:li:activity:(?P<activity_id>\d+)|
            posts/[^/?\#]+|
            learning/[^/?\#]+/(?P<learning_id>[^/?\#]+)
        )
    ",
    )
    .unwrap()
});

pub fn extract_linkedin_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = LINKEDIN_REGEX.captures(trimmed)
        && let Some(id) = captures
            .name("post_id")
            .or_else(|| captures.name("activity_id"))
            .or_else(|| captures.name("learning_id"))
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("linkedin.com")
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

pub fn is_linkedin_url(input: &str) -> bool {
    extract_linkedin_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_linkedin_id() {
        assert_eq!(
            extract_linkedin_id("https://www.linkedin.com/posts/the-mathworks_2_what-is-mathworks-cloud-center-activity-7151241570371948544-4Gu7").unwrap().as_str(),
            "7151241570371948544"
        );
    }
}
