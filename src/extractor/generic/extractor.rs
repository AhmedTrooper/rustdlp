use crate::core::error::{DlpError, Result};
use crate::core::types::{FormatId, MediaType, Protocol, Resolution, VideoId};
use crate::extractor::traits::Extractor;
use crate::models::format::StreamFormat;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use regex::Regex;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use url::Url;

pub struct GenericExtractor {
    http: reqwest::Client,
}

impl Default for GenericExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericExtractor {
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

    pub fn is_direct_media_url(url_str: &str) -> bool {
        if let Ok(parsed) = Url::parse(url_str) {
            let path = parsed.path().to_lowercase();
            matches!(
                path.split('.').next_back(),
                Some(
                    "mp4"
                        | "webm"
                        | "mkv"
                        | "m4a"
                        | "mp3"
                        | "m3u8"
                        | "flv"
                        | "mov"
                        | "avi"
                        | "opus"
                        | "flac"
                        | "wav"
                )
            )
        } else {
            false
        }
    }
}

#[async_trait]
impl Extractor for GenericExtractor {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn can_extract(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    async fn extract(&self, url_str: &str) -> Result<VideoMetadata> {
        let parsed_url = Url::parse(url_str)
            .map_err(|e| DlpError::InvalidUrl(format!("Invalid URL {}: {}", url_str, e)))?;

        // 1. Direct Media Link Handling
        if Self::is_direct_media_url(url_str) {
            let filename = parsed_url
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("video.mp4");

            let ext = filename
                .split('.')
                .next_back()
                .unwrap_or("mp4")
                .to_lowercase();

            let is_audio = matches!(ext.as_str(), "mp3" | "m4a" | "opus" | "flac" | "wav");
            let title = filename
                .strip_suffix(&format!(".{}", ext))
                .unwrap_or(filename)
                .to_string();

            let format = StreamFormat {
                format_id: FormatId::new("direct"),
                url: url_str.to_string(),
                ext: ext.clone(),
                resolution: if is_audio {
                    Resolution::default()
                } else {
                    Resolution {
                        width: Some(1920),
                        height: Some(1080),
                    }
                },
                fps: if is_audio { None } else { Some(30) },
                bitrate: None,
                filesize: None,
                filesize_approx: None,
                media_type: if is_audio {
                    MediaType::AudioOnly
                } else {
                    MediaType::Combined
                },
                protocol: Protocol::Https,
                vcodec: if is_audio { None } else { Some("h264".into()) },
                acodec: Some(if is_audio { ext.clone() } else { "aac".into() }),
                audio_sample_rate: Some(44100),
                audio_channels: Some(2),
                itag: 1,
                quality_label: Some("Direct Media".into()),
                source_client: "generic_direct".into(),
            };

            return Ok(VideoMetadata {
                id: VideoId::new(title.clone()),
                title,
                uploader: parsed_url.host_str().unwrap_or("Generic").to_string(),
                channel_id: None,
                duration: None,
                view_count: None,
                description: None,
                upload_date: None,
                thumbnails: Vec::new(),
                formats: vec![format],
                subtitles: Vec::new(),
                webpage_url: url_str.to_string(),
                is_live: false,
            });
        }

        // 2. HTML Webpage Media Extraction
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

        let response = self.http.get(url_str).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Generic extractor failed with HTTP {}",
                response.status()
            )));
        }

        let html = response.text().await?;

        // Extract metadata
        let title_re = Regex::new(r#"<title>(.*?)</title>"#).unwrap();
        let og_title_re =
            Regex::new(r#"<meta\s+(?:property|name)=["']og:title["']\s+content=["'](.*?)["']"#)
                .unwrap();
        let og_desc_re = Regex::new(
            r#"<meta\s+(?:property|name)=["']og:description["']\s+content=["'](.*?)["']"#,
        )
        .unwrap();
        let og_image_re =
            Regex::new(r#"<meta\s+(?:property|name)=["']og:image["']\s+content=["'](.*?)["']"#)
                .unwrap();

        let title = og_title_re
            .captures(&html)
            .or_else(|| title_re.captures(&html))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Generic Video".to_string());

        let description = og_desc_re
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string());

        let thumbnails = og_image_re
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| vec![m.as_str().trim().to_string()])
            .unwrap_or_default();

        // Extract media sources from HTML5 <video>, <source>, og:video, twitter:player:stream
        let mut formats = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        let media_patterns = [
            r#"<meta\s+(?:property|name)=["']og:video(?::secure_url|:url)?["']\s+content=["'](.*?)["']"#,
            r#"<meta\s+(?:property|name)=["']twitter:player:stream["']\s+content=["'](.*?)["']"#,
            r#"<video[^>]+src=["'](.*?)["']"#,
            r#"<source[^>]+src=["'](.*?)["']"#,
            r#"contentUrl["']\s*:\s*["'](https?://[^"']+)["']"#,
        ];

        for pat in media_patterns {
            if let Ok(re) = Regex::new(pat) {
                for cap in re.captures_iter(&html) {
                    if let Some(src) = cap.get(1) {
                        let raw_src = src.as_str().trim();
                        if let Ok(resolved_url) = parsed_url.join(raw_src) {
                            let url_final = resolved_url.to_string();
                            if !seen_urls.contains(&url_final)
                                && (url_final.contains(".mp4")
                                    || url_final.contains(".m3u8")
                                    || url_final.contains(".webm")
                                    || url_final.contains(".mp3")
                                    || url_final.contains("video"))
                            {
                                seen_urls.insert(url_final.clone());
                                let ext = if url_final.contains(".webm") {
                                    "webm".to_string()
                                } else if url_final.contains(".m3u8") {
                                    "mp4".to_string()
                                } else if url_final.contains(".mp3") {
                                    "mp3".to_string()
                                } else {
                                    "mp4".to_string()
                                };

                                let idx = formats.len() + 1;
                                formats.push(StreamFormat {
                                    format_id: FormatId::new(format!("http-{}", idx)),
                                    url: url_final,
                                    ext,
                                    resolution: Resolution {
                                        width: Some(1280),
                                        height: Some(720),
                                    },
                                    fps: Some(30),
                                    bitrate: Some(1_000_000),
                                    filesize: None,
                                    filesize_approx: None,
                                    media_type: MediaType::Combined,
                                    protocol: Protocol::Https,
                                    vcodec: Some("h264".into()),
                                    acodec: Some("aac".into()),
                                    audio_sample_rate: Some(44100),
                                    audio_channels: Some(2),
                                    itag: idx as u32,
                                    quality_label: Some("HTML5 Stream".into()),
                                    source_client: "generic_html5".into(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No embedded video streams or direct media found at URL: {}",
                url_str
            )));
        }

        let uploader = parsed_url.host_str().unwrap_or("Generic").to_string();

        Ok(VideoMetadata {
            id: VideoId::new(parsed_url.path().trim_matches('/').replace('/', "_")),
            title,
            uploader,
            channel_id: None,
            duration: None,
            view_count: None,
            description,
            upload_date: None,
            thumbnails,
            formats,
            subtitles: Vec::new(),
            webpage_url: url_str.to_string(),
            is_live: false,
        })
    }
}
