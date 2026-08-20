use crate::core::error::{DlpError, Result};
use crate::extractor::soundcloud::parser::SoundCloudParser;
use crate::extractor::soundcloud::url::{extract_soundcloud_id, is_soundcloud_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

pub struct SoundCloudExtractor {
    http: reqwest::Client,
}

impl Default for SoundCloudExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundCloudExtractor {
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
impl Extractor for SoundCloudExtractor {
    fn name(&self) -> &'static str {
        "soundcloud"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_soundcloud_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let track_id = extract_soundcloud_id(url)?;

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

        let response = self.http.get(url).headers(headers.clone()).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "SoundCloud request failed with HTTP {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let mut parsed = SoundCloudParser::parse_html(&html, track_id.as_str());

        // Resolve stream auth URLs if needed
        let mut resolved_formats = Vec::new();
        let client_id = "a3e059563d7fd3372b49b37f00a00bcf"; // Standard public web client_id

        for mut fmt in parsed.formats {
            if fmt.url.contains("api-v2.soundcloud.com/media/") {
                let sep = if fmt.url.contains('?') { "&" } else { "?" };
                let auth_api_url = format!("{}{}{}client_id={}", fmt.url, sep, "", client_id);
                let auth_api_url = auth_api_url.replace("?&", "?");
                if let Ok(res) = self
                    .http
                    .get(&auth_api_url)
                    .headers(headers.clone())
                    .send()
                    .await
                    && let Ok(val) = res.json::<Value>().await
                    && let Some(direct_url) = val.get("url").and_then(|v| v.as_str())
                {
                    fmt.url = direct_url.to_string();
                    resolved_formats.push(fmt);
                }
            } else {
                resolved_formats.push(fmt);
            }
        }

        if resolved_formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable audio tracks found for SoundCloud URL: {}",
                url
            )));
        }

        parsed.formats = resolved_formats;
        let title = parsed
            .title
            .unwrap_or_else(|| format!("SoundCloud track #{}", track_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "SoundCloud Artist".to_string());

        Ok(VideoMetadata {
            id: track_id,
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
