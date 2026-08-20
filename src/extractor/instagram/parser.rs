use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct InstagramParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct InstagramParser;

impl InstagramParser {
    pub fn parse_html(html: &str, video_id: &str) -> InstagramParsedData {
        let mut data = InstagramParsedData::default();

        // 1. Check for embedded video_url in embed HTML
        let video_url_re = Regex::new(r#"class="EmbeddedVideo"[^>]*src="([^"]+)""#).unwrap();
        let video_url_re2 = Regex::new(r#""video_url"\s*:\s*"([^"]+)""#).unwrap();
        let display_url_re = Regex::new(r#""display_url"\s*:\s*"([^"]+)""#).unwrap();
        let username_re = Regex::new(r#""username"\s*:\s*"([^"]+)""#).unwrap();
        let caption_re = Regex::new(r#"class="Caption"[^>]*>([\s\S]*?)</div>"#).unwrap();

        if let Some(cap) = video_url_re
            .captures(html)
            .or_else(|| video_url_re2.captures(html))
            && let Some(url_match) = cap.get(1)
        {
            let video_url = url_match.as_str().replace("\\/", "/").replace("&amp;", "&");
            data.formats.push(StreamFormat {
                format_id: FormatId::new("hd"),
                url: video_url,
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
                itag: 1080,
                quality_label: Some("Instagram HD".to_string()),
                source_client: "instagram_embed".to_string(),
            });
        }

        if let Some(cap) = display_url_re.captures(html)
            && let Some(thumb) = cap.get(1)
        {
            data.thumbnails
                .push(thumb.as_str().replace("\\/", "/").replace("&amp;", "&"));
        }

        if let Some(cap) = username_re.captures(html)
            && let Some(u) = cap.get(1)
        {
            data.uploader = Some(u.as_str().to_string());
        }

        if let Some(cap) = caption_re.captures(html)
            && let Some(c) = cap.get(1)
        {
            let clean_caption = Regex::new(r"<[^>]+>")
                .unwrap()
                .replace_all(c.as_str(), "")
                .trim()
                .to_string();
            data.title = Some(clean_caption.clone());
            data.description = Some(clean_caption);
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
                    bitrate: Some(1_500_000),
                    filesize: None,
                    filesize_approx: None,
                    media_type: MediaType::Combined,
                    protocol: Protocol::Https,
                    vcodec: Some("h264".to_string()),
                    acodec: Some("aac".to_string()),
                    audio_sample_rate: Some(44100),
                    audio_channels: Some(2),
                    itag: 720,
                    quality_label: Some("OpenGraph Video".to_string()),
                    source_client: "instagram_og".to_string(),
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

        if data.title.is_none() {
            data.title = Some(format!("Instagram post #{}", video_id));
        }

        data
    }

    pub fn parse_json(val: &Value, video_id: &str) -> InstagramParsedData {
        let mut data = InstagramParsedData::default();

        let item = val
            .pointer("/items/0")
            .or_else(|| val.pointer("/graphql/shortcode_media"))
            .or_else(|| val.pointer("/data/xdt_shortcode_media"))
            .unwrap_or(val);

        if let Some(user) = item
            .pointer("/user/username")
            .or_else(|| item.pointer("/owner/username"))
            .and_then(|v| v.as_str())
        {
            data.uploader = Some(user.to_string());
        }

        if let Some(caption) = item
            .pointer("/caption/text")
            .or_else(|| item.pointer("/edge_media_to_caption/edges/0/node/text"))
            .and_then(|v| v.as_str())
        {
            data.title = Some(caption.to_string());
            data.description = Some(caption.to_string());
        }

        if let Some(versions) = item.get("video_versions").and_then(|v| v.as_array()) {
            for (idx, v) in versions.iter().enumerate() {
                if let Some(u) = v.get("url").and_then(|v| v.as_str()) {
                    let width = v.get("width").and_then(|v| v.as_u64()).map(|w| w as u32);
                    let height = v.get("height").and_then(|v| v.as_u64()).map(|h| h as u32);
                    let itag = height.unwrap_or((idx + 1) as u32);

                    data.formats.push(StreamFormat {
                        format_id: FormatId::new(format!("video-{}", itag)),
                        url: u.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution { width, height },
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
                        itag,
                        quality_label: height.map(|h| format!("{}p", h)),
                        source_client: "instagram_versions".to_string(),
                    });
                }
            }
        } else if let Some(video_url) = item.get("video_url").and_then(|v| v.as_str()) {
            data.formats.push(StreamFormat {
                format_id: FormatId::new("hd"),
                url: video_url.to_string(),
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
                source_client: "instagram_api".to_string(),
            });
        }

        if data.title.is_none() {
            data.title = Some(format!("Instagram video #{}", video_id));
        }

        data
    }
}
