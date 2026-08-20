use crate::core::error::{DlpError, Result};
use crate::extractor::traits::Extractor;
use crate::extractor::twitch::parser::TwitchParser;
use crate::extractor::twitch::url::{extract_twitch_id, is_twitch_url};
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::{Value, json};

pub struct TwitchExtractor {
    http: reqwest::Client,
}

impl Default for TwitchExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitchExtractor {
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
impl Extractor for TwitchExtractor {
    fn name(&self) -> &'static str {
        "twitch"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_twitch_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_twitch_id(url)?;

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "Client-Id",
            HeaderValue::from_static("ue6666qo983tsx6so1t0vnawi233wa"),
        );

        let gql_body = json!([{
            "operationName": "ShareClipRenderStatus",
            "variables": { "slug": video_id.as_str() },
            "extensions": {
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": "0a02bb974443b576f5579aab0fef1d4b7f44e58a8a256f0c5adfead0db70640f"
                }
            }
        }]);

        let response = self
            .http
            .post("https://gql.twitch.tv/gql")
            .headers(headers)
            .json(&gql_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Twitch GQL request failed with HTTP {}",
                response.status()
            )));
        }

        let json_val: Value = response.json().await?;
        let parsed = TwitchParser::parse_clip_json(&json_val, video_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video formats found for Twitch Clip: {}",
                video_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("Twitch Clip #{}", video_id));
        let uploader = parsed
            .uploader
            .unwrap_or_else(|| "Twitch Creator".to_string());

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
