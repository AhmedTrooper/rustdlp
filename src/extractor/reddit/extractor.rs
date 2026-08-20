use crate::core::error::{DlpError, Result};
use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::extractor::reddit::parser::RedditParser;
use crate::extractor::reddit::url::{extract_reddit_video_id, is_reddit_url};
use crate::extractor::traits::Extractor;
use crate::models::format::StreamFormat;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

pub struct RedditExtractor {
    http: reqwest::Client,
}

impl Default for RedditExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl RedditExtractor {
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
impl Extractor for RedditExtractor {
    fn name(&self) -> &'static str {
        "reddit"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_reddit_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_reddit_video_id(url)?;

        // If direct v.redd.it link without Reddit comments JSON
        if url.contains("v.redd.it") {
            let base_url = format!("https://v.redd.it/{}", video_id.as_str());
            let mut formats = Vec::new();

            for (height, bitrate) in [
                (1080, 4_500_000),
                (720, 2_200_000),
                (480, 1_200_000),
                (360, 600_000),
                (240, 300_000),
            ] {
                formats.push(StreamFormat {
                    format_id: FormatId::new(format!("dash-video-{}", height)),
                    url: format!("{}/DASH_{}.mp4", base_url, height),
                    ext: "mp4".to_string(),
                    resolution: Resolution {
                        width: Some(height * 16 / 9),
                        height: Some(height),
                    },
                    fps: Some(30),
                    bitrate: Some(bitrate),
                    filesize: None,
                    filesize_approx: None,
                    media_type: MediaType::VideoOnly,
                    protocol: Protocol::Https,
                    vcodec: Some("h264".to_string()),
                    acodec: None,
                    audio_sample_rate: None,
                    audio_channels: None,
                    itag: height,
                    quality_label: Some(format!("{}p", height)),
                    source_client: "reddit_direct".to_string(),
                });
            }

            for (audio_tag, bitrate) in
                [("DASH_AUDIO_128.mp4", 128_000), ("DASH_audio.mp4", 64_000)]
            {
                formats.push(StreamFormat {
                    format_id: FormatId::new(if bitrate == 128_000 {
                        "dash-audio-128"
                    } else {
                        "dash-audio-64"
                    }),
                    url: format!("{}/{}", base_url, audio_tag),
                    ext: "m4a".to_string(),
                    resolution: Resolution::default(),
                    fps: None,
                    bitrate: Some(bitrate),
                    filesize: None,
                    filesize_approx: None,
                    media_type: MediaType::AudioOnly,
                    protocol: Protocol::Https,
                    vcodec: None,
                    acodec: Some("aac".to_string()),
                    audio_sample_rate: Some(44100),
                    audio_channels: Some(2),
                    itag: 140,
                    quality_label: Some(format!("{}k audio", bitrate / 1000)),
                    source_client: "reddit_direct".to_string(),
                });
            }

            return Ok(VideoMetadata {
                id: video_id.clone(),
                title: format!("Reddit video #{}", video_id.as_str()),
                uploader: "Reddit User".to_string(),
                channel_id: None,
                duration: None,
                view_count: None,
                description: None,
                upload_date: None,
                thumbnails: vec![format!("{}/DASH_1080.mp4?source=fallback", base_url)],
                formats,
                subtitles: Vec::new(),
                webpage_url: url.to_string(),
                is_live: false,
            });
        }

        let json_url = if url.contains("/comments/") {
            let base = url.split('?').next().unwrap_or(url).trim_end_matches('/');
            format!("{}.json", base)
        } else {
            format!("https://www.reddit.com/comments/{}.json", video_id.as_str())
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        // Initialize session cookie anonymously
        let _ = self
            .http
            .get("https://old.reddit.com/")
            .headers(headers.clone())
            .send()
            .await;

        let response = self.http.get(&json_url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Reddit request failed with HTTP {}",
                response.status()
            )));
        }

        let json_val: Value = response.json().await?;
        let parsed = RedditParser::parse_post_json(&json_val, video_id.as_str());

        if parsed.formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video formats found for Reddit post ID: {}",
                video_id
            )));
        }

        let title = parsed
            .title
            .unwrap_or_else(|| format!("Reddit post #{}", video_id));
        let uploader = parsed.uploader.unwrap_or_else(|| "Reddit User".to_string());

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
