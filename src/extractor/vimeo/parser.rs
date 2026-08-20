use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct VimeoParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct VimeoParser;

impl VimeoParser {
    pub fn parse_config_json(val: &Value, video_id: &str) -> VimeoParsedData {
        let mut data = VimeoParsedData::default();

        if let Some(video) = val.get("video") {
            data.title = video
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            data.duration = video.get("duration").and_then(|v| v.as_u64());

            if let Some(owner) = video.get("owner") {
                data.uploader = owner
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }

            if let Some(thumbs) = video.get("thumbs").and_then(|v| v.as_object()) {
                for (_, url) in thumbs {
                    if let Some(u) = url.as_str() {
                        data.thumbnails.push(u.to_string());
                    }
                }
            }
        }

        let files = val.pointer("/request/files").unwrap_or(val);

        // 1. Progressive MP4 streams (highest priority, no DRM)
        if let Some(progressive) = files.get("progressive").and_then(|v| v.as_array()) {
            for (idx, p) in progressive.iter().enumerate() {
                if let Some(url_str) = p.get("url").and_then(|v| v.as_str()) {
                    let quality = p
                        .get("quality")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let width = p.get("width").and_then(|v| v.as_u64()).map(|w| w as u32);
                    let height = p.get("height").and_then(|v| v.as_u64()).map(|h| h as u32);
                    let fps = p.get("fps").and_then(|v| v.as_u64()).map(|f| f as u32);

                    data.formats.push(StreamFormat {
                        format_id: FormatId::new(format!("http-{}", quality)),
                        url: url_str.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution { width, height },
                        fps,
                        bitrate: None,
                        filesize: None,
                        filesize_approx: None,
                        media_type: MediaType::Combined,
                        protocol: Protocol::Https,
                        vcodec: Some("h264".to_string()),
                        acodec: Some("aac".to_string()),
                        audio_sample_rate: Some(44100),
                        audio_channels: Some(2),
                        itag: height.unwrap_or((idx + 1) as u32),
                        quality_label: Some(quality.to_string()),
                        source_client: "vimeo_progressive".to_string(),
                    });
                }
            }
        }

        // 2. HLS streams (filter out DRM)
        if let Some(hls) = files.get("hls")
            && let Some(cdns) = hls.get("cdns").and_then(|v| v.as_object())
        {
            for (cdn_name, cdn_obj) in cdns {
                let candidate_url = cdn_obj
                    .get("avc_url")
                    .or_else(|| cdn_obj.get("url"))
                    .or_else(|| cdn_obj.get("fallback_url"))
                    .and_then(|v| v.as_str());

                if let Some(url_str) = candidate_url
                    && !url_str.contains("/drm/")
                    && !url_str.contains("drm/cbcs")
                {
                    data.formats.push(StreamFormat {
                        format_id: FormatId::new(format!("hls-{}", cdn_name)),
                        url: url_str.to_string(),
                        ext: "mp4".to_string(),
                        resolution: Resolution {
                            width: Some(1920),
                            height: Some(1080),
                        },
                        fps: Some(60),
                        bitrate: Some(4_500_000),
                        filesize: None,
                        filesize_approx: None,
                        media_type: MediaType::Combined,
                        protocol: Protocol::Hls,
                        vcodec: Some("h264".to_string()),
                        acodec: Some("aac".to_string()),
                        audio_sample_rate: Some(48000),
                        audio_channels: Some(2),
                        itag: 1080,
                        quality_label: Some(format!("HLS ({})", cdn_name)),
                        source_client: "vimeo_hls".to_string(),
                    });
                    break;
                }
            }
        }

        if data.title.is_none() {
            data.title = Some(format!("Vimeo video #{}", video_id));
        }

        data
    }
}
