use crate::core::error::{DlpError, Result};
use crate::extractor::tiktok::parser::TikTokParser;
use crate::extractor::tiktok::url::{extract_tiktok_video_id, is_tiktok_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};

pub struct TikTokExtractor {
    http: reqwest::Client,
}

impl Default for TikTokExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TikTokExtractor {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .cookie_store(true)
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
impl Extractor for TikTokExtractor {
    fn name(&self) -> &'static str {
        "tiktok"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_tiktok_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
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

        // Follow any short link redirects (e.g. vm.tiktok.com / vt.tiktok.com)
        let response = self.http.get(url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "TikTok request failed with HTTP {}",
                response.status()
            )));
        }

        let final_url = response.url().to_string();
        let video_id =
            extract_tiktok_video_id(&final_url).or_else(|_| extract_tiktok_video_id(url))?;

        let html = response.text().await?;
        let parsed = TikTokParser::parse_html(&html, video_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video streams found for TikTok ID: {}",
                video_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("TikTok video #{}", video_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "TikTok Creator".to_string());

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
            webpage_url: final_url,
            is_live: false,
        })
    }
}
