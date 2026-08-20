use crate::core::error::{DlpError, Result};
use crate::extractor::pinterest::parser::PinterestParser;
use crate::extractor::pinterest::url::{extract_pinterest_id, is_pinterest_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};

pub struct PinterestExtractor {
    http: reqwest::Client,
}

impl Default for PinterestExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PinterestExtractor {
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
impl Extractor for PinterestExtractor {
    fn name(&self) -> &'static str {
        "pinterest"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_pinterest_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let pin_id = extract_pinterest_id(url)?;
        let pin_url = format!("https://www.pinterest.com/pin/{}/", pin_id.as_str());

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

        let response = self.http.get(&pin_url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Pinterest request failed with HTTP {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let parsed = PinterestParser::parse_html(&html, pin_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video formats found for Pinterest Pin: {}",
                pin_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("Pinterest Pin #{}", pin_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "Pinterest User".to_string());

        Ok(VideoMetadata {
            id: pin_id,
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
            webpage_url: pin_url,
            is_live: false,
        })
    }
}
