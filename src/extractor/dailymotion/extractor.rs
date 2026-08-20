use crate::core::error::{DlpError, Result};
use crate::extractor::dailymotion::parser::DailymotionParser;
use crate::extractor::dailymotion::url::{extract_dailymotion_id, is_dailymotion_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::{Value, json};

pub struct DailymotionExtractor {
    http: reqwest::Client,
}

impl Default for DailymotionExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl DailymotionExtractor {
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
impl Extractor for DailymotionExtractor {
    fn name(&self) -> &'static str {
        "dailymotion"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_dailymotion_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_dailymotion_id(url)?;

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://www.dailymotion.com/"),
        );

        // 1. Try Player Metadata API
        let meta_url = format!(
            "https://www.dailymotion.com/player/metadata/video/{}",
            video_id.as_str()
        );

        if let Ok(response) = self
            .http
            .get(&meta_url)
            .headers(headers.clone())
            .send()
            .await
            && response.status().is_success()
            && let Ok(json_val) = response.json::<Value>().await
        {
            let parsed = DailymotionParser::parse_metadata_json(&json_val, video_id.as_str());
            if !parsed.formats.is_empty() {
                let title = parsed
                    .title
                    .unwrap_or_else(|| format!("Dailymotion video #{}", video_id));
                let uploader = parsed
                    .uploader
                    .unwrap_or_else(|| "Dailymotion User".to_string());

                return Ok(VideoMetadata {
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
                });
            }
        }

        // 2. Fallback: GraphQL API
        let gql_url = "https://graphql.dailymotion.com/";
        let gql_body = json!({
            "query": "query VideoQuery($id: String!) { video(xid: $id) { id title description duration owner { username screenname } posterUrl } }",
            "variables": { "id": video_id.as_str() }
        });

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Ok(response) = self
            .http
            .post(gql_url)
            .headers(headers)
            .json(&gql_body)
            .send()
            .await
            && response.status().is_success()
            && let Ok(gql_val) = response.json::<Value>().await
            && let Some(video_node) = gql_val.pointer("/data/video")
        {
            let parsed = DailymotionParser::parse_metadata_json(video_node, video_id.as_str());
            let title = parsed
                .title
                .unwrap_or_else(|| format!("Dailymotion video #{}", video_id));
            let uploader = parsed
                .uploader
                .unwrap_or_else(|| "Dailymotion User".to_string());

            return Ok(VideoMetadata {
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
            });
        }

        Err(DlpError::VideoUnavailable(format!(
            "No downloadable video formats found for Dailymotion video ID: {}",
            video_id
        )))
    }
}
