use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct TwitchParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct TwitchParser;

impl TwitchParser {
    pub fn parse_clip_json(val: &Value, clip_id: &str) -> TwitchParsedData {
        let mut data = TwitchParsedData::default();

        let clip = val
            .pointer("/0/data/clip")
            .or_else(|| val.pointer("/data/clip"))
            .unwrap_or(val);

        data.title = clip
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        data.duration = clip.get("durationSeconds").and_then(|v| v.as_u64());

        if let Some(broadcaster) = clip.get("broadcaster") {
            data.uploader = broadcaster
                .get("displayName")
                .or_else(|| broadcaster.get("login"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        if let Some(thumb) = clip.get("thumbnailURL").and_then(|v| v.as_str()) {
            data.thumbnails.push(thumb.to_string());
        }

        let sig = clip
            .pointer("/playbackAccessToken/signature")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let token = clip
            .pointer("/playbackAccessToken/value")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(assets) = clip.get("assets").and_then(|v| v.as_array()) {
            for asset in assets {
                if let Some(qualities) = asset.get("videoQualities").and_then(|v| v.as_array()) {
                    for (idx, q) in qualities.iter().enumerate() {
                        if let Some(src_url) = q.get("sourceURL").and_then(|v| v.as_str()) {
                            let quality = q
                                .get("quality")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let fps = q
                                .get("frameRate")
                                .and_then(|v| v.as_f64())
                                .map(|f| f as u32);
                            let height = quality.parse::<u32>().ok().unwrap_or(1080);

                            let authenticated_url = if !sig.is_empty() && !token.is_empty() {
                                let sep = if src_url.contains('?') { "&" } else { "?" };
                                format!("{}{}sig={}&token={}", src_url, sep, sig, token)
                            } else {
                                src_url.to_string()
                            };

                            let idx_num = idx + 1;
                            data.formats.push(StreamFormat {
                                format_id: FormatId::new(format!("{}-{}", quality, idx_num)),
                                url: authenticated_url,
                                ext: "mp4".to_string(),
                                resolution: Resolution {
                                    width: Some(height * 16 / 9),
                                    height: Some(height),
                                },
                                fps,
                                bitrate: Some((height as u64) * 3500),
                                filesize: None,
                                filesize_approx: None,
                                media_type: MediaType::Combined,
                                protocol: Protocol::Https,
                                vcodec: Some("h264".to_string()),
                                acodec: Some("aac".to_string()),
                                audio_sample_rate: Some(44100),
                                audio_channels: Some(2),
                                itag: height,
                                quality_label: Some(format!("{}p", height)),
                                source_client: "twitch_clip".to_string(),
                            });
                        }
                    }
                }
            }
        }

        if data.title.is_none() {
            data.title = Some(format!("Twitch Clip #{}", clip_id));
        }

        data
    }
}
