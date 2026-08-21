use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static YOUTUBE_VIDEO_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?x)
        (?:https?://)?
        (?:
            (?:www|m|music|gaming)\.(?:youtube|youtube-nocookie)\.com/(?:watch\?.*?\bv=|embed/|v/|e/|shorts/|live/|clip/|movie/)|
            youtu\.be/|
            (?:www\.)?youtube\.googleapis\.com/v/|
            (?:www\.)?(?:hooktube|pwnyoutube|vid\.plus|inv\.tux\.pizza|yewtu\.be|invidious\.[a-z.]+)/(?:watch\?.*?\bv=|embed/|v/|shorts/)
        )
        (?P<id>[a-zA-Z0-9_-]{11})
    ").unwrap()
});

pub fn extract_youtube_video_id(input: &str) -> Result<VideoId> {
    let trimmed = input.trim();

    if let Some(cap) = YOUTUBE_VIDEO_REGEX.captures(trimmed)
        && let Some(id) = cap.name("id")
    {
        return Ok(VideoId::new(id.as_str()));
    }

    if let Ok(parsed) = Url::parse(trimmed) {
        if let Some(query) = parsed.query() {
            for pair in query.split('&') {
                if let Some((k, v)) = pair.split_once('=')
                    && k == "v"
                    && v.len() == 11
                {
                    return Ok(VideoId::new(v));
                }
            }
        }

        if let Some(domain) = parsed.domain()
            && (domain.contains("youtube.com")
                || domain.contains("youtu.be")
                || domain.contains("youtube-nocookie.com")
                || domain.contains("hooktube")
                || domain.contains("invidious"))
        {
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            if let Some(last) = segments.into_iter().rev().find(|s| s.len() == 11) {
                return Ok(VideoId::new(last));
            }
        }
    }

    Err(DlpError::InvalidUrl(input.to_string()))
}

pub fn is_youtube_url(input: &str) -> bool {
    extract_youtube_video_id(input).is_ok()
        || input.contains("youtube.com")
        || input.contains("youtu.be")
        || input.contains("youtube-nocookie.com")
        || input.contains("hooktube.com")
        || input.contains("invidious")
}

pub fn extract_playlist_id(url: &str) -> Option<String> {
    crate::extractor::youtube::tab::extract_playlist_id(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_youtube_video_id() {
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://youtu.be/dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://music.youtube.com/watch?v=dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://hooktube.com/watch?v=dQw4w9WgXcQ")
                .unwrap()
                .as_str(),
            "dQw4w9WgXcQ"
        );
    }
}
