use crate::core::error::{DlpError, Result};
use crate::extractor::linkedin::parser::LinkedInParser;
use crate::extractor::linkedin::url::{extract_linkedin_id, is_linkedin_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};

pub struct LinkedInExtractor {
    http: reqwest::Client,
}

impl Default for LinkedInExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkedInExtractor {
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
impl Extractor for LinkedInExtractor {
    fn name(&self) -> &'static str {
        "linkedin"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_linkedin_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_linkedin_id(url)?;

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

        let response = self.http.get(url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "LinkedIn request failed with HTTP {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let parsed = LinkedInParser::parse_html(&html, video_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video streams found for LinkedIn post ID: {}",
                video_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("LinkedIn post #{}", video_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "LinkedIn User".to_string());

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
            subtitles: parsed.subtitles,
            webpage_url: url.to_string(),
            is_live: false,
        })
    }
}
