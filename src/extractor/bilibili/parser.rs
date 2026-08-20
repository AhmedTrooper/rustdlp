use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct BilibiliViewInfo {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub cid: Option<u64>,
}

pub struct BilibiliParser;

impl BilibiliParser {
    pub fn parse_view_json(val: &Value) -> BilibiliViewInfo {
        let data = val.get("data").unwrap_or(val);
        let title = data
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let description = data
            .get("desc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let duration = data.get("duration").and_then(|v| v.as_u64());
        let uploader = data
            .pointer("/owner/name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let cid = data
            .get("cid")
            .and_then(|v| v.as_u64())
            .or_else(|| data.pointer("/pages/0/cid").and_then(|v| v.as_u64()));

        let mut thumbnails = Vec::new();
        if let Some(pic) = data.get("pic").and_then(|v| v.as_str()) {
            thumbnails.push(pic.to_string());
        }

        BilibiliViewInfo {
            title,
            description,
            uploader,
            duration,
            thumbnails,
            cid,
        }
    }

    pub fn parse_playurl_json(val: &Value, _bvid: &str) -> Vec<StreamFormat> {
        let mut formats = Vec::new();
        let data = val.get("data").or_else(|| val.get("result")).unwrap_or(val);

        // 1. Parse DASH streams
        if let Some(dash) = data.get("dash") {
            if let Some(videos) = dash.get("video").and_then(|v| v.as_array()) {
                for (idx, v) in videos.iter().enumerate() {
                    let url_str = v
                        .get("baseUrl")
                        .or_else(|| v.get("base_url"))
                        .and_then(|s| s.as_str());
                    if let Some(u) = url_str {
                        let width = v.get("width").and_then(|n| n.as_u64()).map(|w| w as u32);
                        let height = v.get("height").and_then(|n| n.as_u64()).map(|h| h as u32);
                        let id_num = v
                            .get("id")
                            .and_then(|n| n.as_u64())
                            .unwrap_or((idx + 1) as u64);
                        let bitrate = v.get("bandwidth").and_then(|n| n.as_u64());

                        formats.push(StreamFormat {
                            format_id: FormatId::new(format!("dash-video-{}", id_num)),
                            url: u.to_string(),
                            ext: "mp4".to_string(),
                            resolution: Resolution { width, height },
                            fps: Some(30),
                            bitrate,
                            filesize: None,
                            filesize_approx: None,
                            media_type: MediaType::VideoOnly,
                            protocol: Protocol::Https,
                            vcodec: Some("h264".to_string()),
                            acodec: None,
                            audio_sample_rate: None,
                            audio_channels: None,
                            itag: height.unwrap_or(id_num as u32),
                            quality_label: height.map(|h| format!("{}p", h)),
                            source_client: "bilibili_dash".to_string(),
                        });
                    }
                }
            }

            if let Some(audios) = dash.get("audio").and_then(|v| v.as_array()) {
                for (idx, a) in audios.iter().enumerate() {
                    let url_str = a
                        .get("baseUrl")
                        .or_else(|| a.get("base_url"))
                        .and_then(|s| s.as_str());
                    if let Some(u) = url_str {
                        let id_num = a
                            .get("id")
                            .and_then(|n| n.as_u64())
                            .unwrap_or((idx + 1) as u64);
                        let bitrate = a
                            .get("bandwidth")
                            .and_then(|n| n.as_u64())
                            .unwrap_or(128_000);

                        formats.push(StreamFormat {
                            format_id: FormatId::new(format!("dash-audio-{}", id_num)),
                            url: u.to_string(),
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
                            source_client: "bilibili_dash_audio".to_string(),
                        });
                    }
                }
            }
        }

        // 2. Parse Durl (progressive MP4 fallback)
        if formats.is_empty()
            && let Some(durl) = data.get("durl").and_then(|v| v.as_array())
        {
            for (idx, d) in durl.iter().enumerate() {
                if let Some(u) = d.get("url").and_then(|v| v.as_str()) {
                    let size = d.get("size").and_then(|v| v.as_u64());
                    let idx_num = idx + 1;
                    formats.push(StreamFormat {
                        format_id: FormatId::new(format!("durl-{}", idx_num)),
                        url: u.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution {
                            width: Some(1920),
                            height: Some(1080),
                        },
                        fps: Some(30),
                        bitrate: Some(2_500_000),
                        filesize: size,
                        filesize_approx: None,
                        media_type: MediaType::Combined,
                        protocol: Protocol::Https,
                        vcodec: Some("h264".to_string()),
                        acodec: Some("aac".to_string()),
                        audio_sample_rate: Some(44100),
                        audio_channels: Some(2),
                        itag: 1080,
                        quality_label: Some("HD".to_string()),
                        source_client: "bilibili_durl".to_string(),
                    });
                }
            }
        }

        formats
    }
}
