use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::extractor::facebook::parser::FacebookParser;
use crate::extractor::facebook::url::{extract_facebook_video_id, is_facebook_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};

pub struct FacebookExtractor {
    http: reqwest::Client,
}

impl Default for FacebookExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FacebookExtractor {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_http_client(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn extract_by_id(&self, video_id: &VideoId) -> Result<VideoMetadata> {
        let watch_url = format!("https://www.facebook.com/watch/?v={}", video_id.as_str());

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-us,en;q=0.5"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));

        let response = self.http.get(&watch_url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Facebook request failed with HTTP {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let parsed = FacebookParser::parse_webpage(&html, video_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video formats found for Facebook video ID: {}",
                video_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("Facebook video #{}", video_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "Facebook User".to_string());

        let mut formats = parsed.formats;
        // Sort formats: audio first (by bitrate), then video (by resolution, bitrate)
        formats.sort_by(|a, b| match (a.is_audio_only(), b.is_audio_only()) {
            (true, true) => a.effective_bitrate().cmp(&b.effective_bitrate()),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => {
                let res_cmp = a.height().cmp(&b.height());
                if res_cmp != std::cmp::Ordering::Equal {
                    res_cmp
                } else {
                    a.effective_bitrate().cmp(&b.effective_bitrate())
                }
            }
        });

        Ok(VideoMetadata {
            id: video_id.clone(),
            title,
            uploader,
            channel_id: None,
            duration: parsed.duration,
            view_count: None,
            description: parsed.description,
            upload_date: None,
            thumbnails: parsed.thumbnails,
            formats,
            subtitles: Vec::new(),
            webpage_url: watch_url,
            is_live: false,
        })
    }
}

#[async_trait]
impl Extractor for FacebookExtractor {
    fn name(&self) -> &'static str {
        "facebook"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_facebook_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_facebook_video_id(url)?;
        self.extract_by_id(&video_id).await
    }
}
