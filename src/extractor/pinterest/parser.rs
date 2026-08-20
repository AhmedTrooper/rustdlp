use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct PinterestParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct PinterestParser;

impl PinterestParser {
    pub fn parse_html(html: &str, pin_id: &str) -> PinterestParsedData {
        let mut data = PinterestParsedData::default();

        // 1. JSON-LD Snippet
        let ld_re =
            Regex::new(r#"<script[^>]+type="application/ld\+json"[^>]*>([\s\S]*?)</script>"#)
                .unwrap();
        for cap in ld_re.captures_iter(html) {
            if let Some(json_raw) = cap.get(1)
                && let Ok(val) = serde_json::from_str::<Value>(json_raw.as_str())
            {
                if let Some(title) = val
                    .get("name")
                    .or_else(|| val.get("headline"))
                    .and_then(|v| v.as_str())
                {
                    data.title = Some(title.to_string());
                }
                if let Some(desc) = val.get("description").and_then(|v| v.as_str()) {
                    data.description = Some(desc.to_string());
                }
                if let Some(creator) = val
                    .get("creator")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                {
                    data.uploader = Some(creator.to_string());
                }
                if let Some(content_url) = val.get("contentUrl").and_then(|v| v.as_str()) {
                    data.formats.push(StreamFormat {
                        format_id: FormatId::new("ld-video"),
                        url: content_url.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution {
                            width: Some(1080),
                            height: Some(1920),
                        },
                        fps: Some(30),
                        bitrate: Some(2_500_000),
                        filesize: None,
                        filesize_approx: None,
                        media_type: MediaType::Combined,
                        protocol: Protocol::Https,
                        vcodec: Some("h264".to_string()),
                        acodec: Some("aac".to_string()),
                        audio_sample_rate: Some(44100),
                        audio_channels: Some(2),
                        itag: 1080,
                        quality_label: Some("HD".to_string()),
                        source_client: "pinterest_ld".to_string(),
                    });
                }
            }
        }

        // 2. OpenGraph Fallbacks
        if data.formats.is_empty() {
            let og_video_re = Regex::new(r#"<meta\s+(?:property|name)=["']og:video(?::secure_url|:url)?["']\s+content=["'](.*?)["']"#).unwrap();
            if let Some(cap) = og_video_re.captures(html)
                && let Some(v_url) = cap.get(1)
            {
                data.formats.push(StreamFormat {
                    format_id: FormatId::new("og-video"),
                    url: v_url.as_str().replace("&amp;", "&"),
                    ext: "mp4".to_string(),
                    resolution: Resolution {
                        width: Some(1080),
                        height: Some(1920),
                    },
                    fps: Some(30),
                    bitrate: Some(2_000_000),
                    filesize: None,
                    filesize_approx: None,
                    media_type: MediaType::Combined,
                    protocol: Protocol::Https,
                    vcodec: Some("h264".to_string()),
                    acodec: Some("aac".to_string()),
                    audio_sample_rate: Some(44100),
                    audio_channels: Some(2),
                    itag: 720,
                    quality_label: Some("HD".to_string()),
                    source_client: "pinterest_og".to_string(),
                });
            }
        }

        if data.title.is_none() {
            let og_title_re =
                Regex::new(r#"<meta\s+(?:property|name)=["']og:title["']\s+content=["'](.*?)["']"#)
                    .unwrap();
            data.title = og_title_re
                .captures(html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
        }

        let og_img_re =
            Regex::new(r#"<meta\s+(?:property|name)=["']og:image["']\s+content=["'](.*?)["']"#)
                .unwrap();
        if let Some(thumb) = og_img_re.captures(html).and_then(|c| c.get(1)) {
            data.thumbnails.push(thumb.as_str().to_string());
        }

        if data.title.is_none() {
            data.title = Some(format!("Pinterest Pin #{}", pin_id));
        }

        data
    }
}
