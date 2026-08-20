use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static FB_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?x)
        (?:https?://)?
        (?:[\w-]+\.)?(?:facebook\.com|fb\.watch|fb\.com|facebookwkhpilnemxj7asaniu7vnjjbiltxjqhye3mhbshg7kx5tfyd\.onion)/
        (?:
            watch/?\?(?:.*?&)?v=|
            watch/live/?\?(?:.*?&)?v=|
            video/video\.php\?(?:.*?&)?v=|
            video\.php\?(?:.*?&)?v=|
            story\.php\?(?:.*?&)?story_fbid=|
            permalink\.php\?(?:.*?&)?story_fbid=|
            reel/|
            reels/|
            (?:[^/]+)/videos/(?:[^/]+/)?|
            (?:[^/]+)/posts/|
            groups/(?:[^/]+)/(?:permalink|posts)/(?:[\da-f]+/)?|
            share/v/|
            share/r/|
            share/p/
        )
        (?P<id>pfbid[A-Za-z0-9]+|\d+)
    ").unwrap()
});

static RAW_FB_ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:pfbid[A-Za-z0-9]+|\d{10,25})$").unwrap());

pub fn extract_facebook_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if RAW_FB_ID_REGEX.is_match(trimmed) {
        return Ok(VideoId::new(trimmed));
    }

    if let Some(captures) = FB_URL_REGEX.captures(trimmed)
        && let Some(id) = captures.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && let Some(domain) = parsed.domain()
        && (domain.contains("facebook.com")
            || domain.contains("fb.watch")
            || domain.contains("fb.com"))
    {
        for (k, v) in parsed.query_pairs() {
            if (k == "v" || k == "video_id" || k == "story_fbid") && !v.is_empty() {
                return Ok(VideoId::new(v.as_ref()));
            }
        }

        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.collect())
            .unwrap_or_default();
        for (i, seg) in segments.iter().enumerate() {
            if matches!(*seg, "reel" | "reels" | "videos" | "posts" | "v" | "r")
                && let Some(next) = segments.get(i + 1)
                && !next.is_empty()
            {
                return Ok(VideoId::new(*next));
            }
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_facebook_url(input: &str) -> bool {
    extract_facebook_video_id(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_facebook_video_id() {
        assert_eq!(
            extract_facebook_video_id("https://www.facebook.com/watch/?v=10154383743583686")
                .unwrap()
                .as_str(),
            "10154383743583686"
        );
        assert_eq!(
            extract_facebook_video_id("https://www.facebook.com/reel/10154383743583686")
                .unwrap()
                .as_str(),
            "10154383743583686"
        );
        assert_eq!(
            extract_facebook_video_id("https://www.facebook.com/gov.sg/videos/10154383743583686/")
                .unwrap()
                .as_str(),
            "10154383743583686"
        );
        assert_eq!(
            extract_facebook_video_id("https://www.facebook.com/video.php?v=10154383743583686")
                .unwrap()
                .as_str(),
            "10154383743583686"
        );
        assert_eq!(
            extract_facebook_video_id("10154383743583686")
                .unwrap()
                .as_str(),
            "10154383743583686"
        );
    }
}
