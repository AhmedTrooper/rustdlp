use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static SC_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?soundcloud\.com/
        (?P<user>[\w\d_-]+)/
        (?P<track>[\w\d_-]+)
    ",
    )
    .unwrap()
});

pub fn extract_soundcloud_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(captures) = SC_REGEX.captures(trimmed)
        && let (Some(u), Some(t)) = (captures.name("user"), captures.name("track"))
    {
        return Ok(VideoId::new(format!("{}/{}", u.as_str(), t.as_str())));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && domain.contains("soundcloud.com")
    {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        if segments.len() >= 2 {
            return Ok(VideoId::new(format!("{}/{}", segments[0], segments[1])));
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_soundcloud_url(input: &str) -> bool {
    extract_soundcloud_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_soundcloud_id() {
        assert_eq!(
            extract_soundcloud_id("https://soundcloud.com/octobersveryown/drake-gods-plan")
                .unwrap()
                .as_str(),
            "octobersveryown/drake-gods-plan"
        );
    }
}
