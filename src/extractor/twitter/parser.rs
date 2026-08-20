use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct TwitterParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct TwitterParser;

impl TwitterParser {
    pub fn parse_vxtwitter_json(val: &Value, tweet_id: &str) -> TwitterParsedData {
        let mut data = TwitterParsedData::default();

        data.title = val
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        data.description = data.title.clone();
        data.uploader = val
            .get("user_name")
            .or_else(|| val.get("user_screen_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(media_url) = val
            .get("media_url")
            .or_else(|| val.get("video_url"))
            .and_then(|v| v.as_str())
        {
            data.formats.push(StreamFormat {
                format_id: FormatId::new("hd"),
                url: media_url.to_string(),
                ext: "mp4".to_string(),
                resolution: Resolution {
                    width: Some(1920),
                    height: Some(1080),
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
                source_client: "twitter_direct".to_string(),
            });
        }

        if let Some(media_list) = val.get("media_extended").and_then(|v| v.as_array()) {
            for (idx, m) in media_list.iter().enumerate() {
                if let Some(u) = m.get("url").and_then(|v| v.as_str())
                    && (m.get("type").and_then(|v| v.as_str()) == Some("video")
                        || u.contains(".mp4"))
                {
                    data.formats.push(StreamFormat {
                        format_id: FormatId::new(format!("video-{}", idx + 1)),
                        url: u.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution {
                            width: m.get("width").and_then(|v| v.as_u64()).map(|w| w as u32),
                            height: m.get("height").and_then(|v| v.as_u64()).map(|h| h as u32),
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
                        itag: (idx + 1) as u32,
                        quality_label: Some(format!("Quality {}", idx + 1)),
                        source_client: "twitter_media".to_string(),
                    });
                }
            }
        }

        if data.title.is_none() {
            data.title = Some(format!("Tweet #{}", tweet_id));
        }

        data
    }
}
