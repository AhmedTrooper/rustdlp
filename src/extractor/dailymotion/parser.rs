use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct DailymotionParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct DailymotionParser;

impl DailymotionParser {
    pub fn parse_metadata_json(val: &Value, video_id: &str) -> DailymotionParsedData {
        let title = val
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let description = val
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let duration = val.get("duration").and_then(|v| v.as_u64());

        let uploader = val.get("owner").and_then(|owner| {
            owner
                .get("screenname")
                .or_else(|| owner.get("username"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        let mut thumbnails = Vec::new();
        if let Some(posters) = val.get("posters").and_then(|v| v.as_object()) {
            for (_, url) in posters {
                if let Some(u) = url.as_str() {
                    thumbnails.push(u.to_string());
                }
            }
        }
        if let Some(poster) = val.get("posterUrl").and_then(|v| v.as_str()) {
            thumbnails.push(poster.to_string());
        }

        let mut formats = Vec::new();

        // 1. Qualities extraction (HLS master playlist and direct qualities)
        if let Some(qualities) = val.get("qualities").and_then(|v| v.as_object()) {
            for (q_name, q_arr) in qualities {
                if let Some(arr) = q_arr.as_array() {
                    for item in arr {
                        if let Some(url_str) = item.get("url").and_then(|v| v.as_str()) {
                            let is_hls = url_str.contains(".m3u8");
                            let height = match q_name.as_str() {
                                "1080" => Some(1080),
                                "720" => Some(720),
                                "480" => Some(480),
                                "380" | "360" => Some(360),
                                "240" => Some(240),
                                _ => None,
                            };

                            let idx = formats.len() + 1;
                            formats.push(StreamFormat {
                                format_id: FormatId::new(format!("{}-{}", q_name, idx)),
                                url: url_str.to_string(),
                                ext: "mp4".to_string(),
                                resolution: Resolution {
                                    width: height.map(|h| h * 16 / 9),
                                    height,
                                },
                                fps: Some(30),
                                bitrate: height.map(|h| (h as u64) * 3000),
                                filesize: None,
                                filesize_approx: None,
                                media_type: MediaType::Combined,
                                protocol: if is_hls {
                                    Protocol::Hls
                                } else {
                                    Protocol::Https
                                },
                                vcodec: Some("h264".to_string()),
                                acodec: Some("aac".to_string()),
                                audio_sample_rate: Some(44100),
                                audio_channels: Some(2),
                                itag: height.unwrap_or(idx as u32),
                                quality_label: Some(q_name.to_string()),
                                source_client: "dailymotion_player".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 2. Stream url in qualities edges (GraphQL response format)
        if let Some(edges) = val.pointer("/qualities/edges").and_then(|v| v.as_array()) {
            for (idx, edge) in edges.iter().enumerate() {
                if let Some(node) = edge.get("node")
                    && let Some(url_str) = node.get("url").and_then(|v| v.as_str())
                {
                    let is_hls = url_str.contains(".m3u8");
                    let idx_num = idx + 1;
                    formats.push(StreamFormat {
                        format_id: FormatId::new(format!("graphql-{}", idx_num)),
                        url: url_str.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution {
                            width: Some(1920),
                            height: Some(1080),
                        },
                        fps: Some(30),
                        bitrate: Some(3_000_000),
                        filesize: None,
                        filesize_approx: None,
                        media_type: MediaType::Combined,
                        protocol: if is_hls {
                            Protocol::Hls
                        } else {
                            Protocol::Https
                        },
                        vcodec: Some("h264".to_string()),
                        acodec: Some("aac".to_string()),
                        audio_sample_rate: Some(44100),
                        audio_channels: Some(2),
                        itag: 1080,
                        quality_label: Some("HD".to_string()),
                        source_client: "dailymotion_graphql".to_string(),
                    });
                }
            }
        }

        DailymotionParsedData {
            title: title.or_else(|| Some(format!("Dailymotion video #{}", video_id))),
            description,
            uploader,
            duration,
            thumbnails,
            formats,
        }
    }
}
