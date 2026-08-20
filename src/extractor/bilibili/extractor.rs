use crate::core::error::{DlpError, Result};
use crate::extractor::bilibili::parser::BilibiliParser;
use crate::extractor::bilibili::url::{extract_bilibili_id, is_bilibili_url};
use crate::extractor::traits::Extractor;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

pub struct BilibiliExtractor {
    http: reqwest::Client,
}

impl Default for BilibiliExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl BilibiliExtractor {
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
impl Extractor for BilibiliExtractor {
    fn name(&self) -> &'static str {
        "bilibili"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_bilibili_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_bilibili_id(url)?;

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
            HeaderValue::from_static("https://www.bilibili.com/"),
        );

        // 1. Fetch video details
        let view_url = format!(
            "https://api.bilibili.com/x/web-interface/view?bvid={}",
            video_id.as_str()
        );
        let response = self
            .http
            .get(&view_url)
            .headers(headers.clone())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Bilibili view API request failed with HTTP {}",
                response.status()
            )));
        }

        let view_json: Value = response.json().await?;
        let view_info = BilibiliParser::parse_view_json(&view_json);

        let cid = match view_info.cid {
            Some(c) => c,
            None => {
                return Err(DlpError::VideoUnavailable(format!(
                    "Could not find CID for Bilibili video {}",
                    video_id
                )));
            }
        };

        // 2. Fetch playurl with fnval=16 (DASH)
        let playurl = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=116&fnval=16&fnver=0&fourk=1",
            video_id.as_str(),
            cid
        );

        let play_resp = self.http.get(&playurl).headers(headers).send().await?;
        if !play_resp.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Bilibili playurl API failed with HTTP {}",
                play_resp.status()
            )));
        }

        let play_json: Value = play_resp.json().await?;
        let formats = BilibiliParser::parse_playurl_json(&play_json, video_id.as_str());

        if formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video formats found for Bilibili video ID: {}",
                video_id
            )));
        }

        let final_title = view_info
            .title
            .unwrap_or_else(|| format!("Bilibili video #{}", video_id));
        let final_uploader = view_info
            .uploader
            .unwrap_or_else(|| "Bilibili Creator".to_string());

        Ok(VideoMetadata {
            id: video_id,
            title: final_title,
            uploader: final_uploader,
            channel_id: None,
            duration: view_info.duration,
            view_count: None,
            description: view_info.description,
            upload_date: None,
            thumbnails: view_info.thumbnails,
            formats,
            subtitles: Vec::new(),
            webpage_url: url.to_string(),
            is_live: false,
        })
    }
}
