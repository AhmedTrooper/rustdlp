use crate::core::error::{DlpError, Result};
use crate::extractor::traits::Extractor;
use crate::extractor::twitter::parser::TwitterParser;
use crate::extractor::twitter::url::{extract_twitter_video_id, is_twitter_url};
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

pub struct TwitterExtractor {
    http: reqwest::Client,
}

impl Default for TwitterExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitterExtractor {
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
}

#[async_trait]
impl Extractor for TwitterExtractor {
    fn name(&self) -> &'static str {
        "twitter"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_twitter_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_twitter_video_id(url)?;
        let api_url = format!(
            "https://api.vxtwitter.com/Twitter/status/{}",
            video_id.as_str()
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let response = self.http.get(&api_url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Twitter API request failed with HTTP {}",
                response.status()
            )));
        }

        let json_val: Value = response.json().await?;
        let parsed = TwitterParser::parse_vxtwitter_json(&json_val, video_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video streams found in Tweet ID: {}",
                video_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("Tweet #{}", video_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "Twitter User".to_string());

        Ok(VideoMetadata {
            id: video_id,
            title,
            uploader,
            channel_id: None,
            duration: parsed.duration,
            view_count: None,
            description: parsed.description,
            upload_date: None,
            thumbnails: parsed.thumbnails,
            formats: parsed.formats,
            subtitles: Vec::new(),
            webpage_url: url.to_string(),
            is_live: false,
        })
    }
}
