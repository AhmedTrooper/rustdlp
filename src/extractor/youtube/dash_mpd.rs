use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;

pub struct YoutubeDashMpdParser;

impl YoutubeDashMpdParser {
    pub fn parse(xml: &str, mpd_url: &str) -> Vec<StreamFormat> {
        let mut formats = Vec::new();

        let rep_re = Regex::new(r#"(?s)<Representation\s+([^>]+)>(.*?)</Representation>"#).unwrap();
        let base_url_re = Regex::new(r#"<BaseURL[^>]*>(.*?)</BaseURL>"#).unwrap();

        for cap in rep_re.captures_iter(xml) {
            let attr_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let inner_content = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let id = extract_attr(attr_str, "id").unwrap_or_else(|| "0".to_string());
            let width = extract_attr(attr_str, "width").and_then(|s| s.parse::<u32>().ok());
            let height = extract_attr(attr_str, "height").and_then(|s| s.parse::<u32>().ok());
            let bandwidth = extract_attr(attr_str, "bandwidth").and_then(|s| s.parse::<u64>().ok());
            let mime_type = extract_attr(attr_str, "mimeType");
            let codecs = extract_attr(attr_str, "codecs");
            let audio_sampling_rate =
                extract_attr(attr_str, "audioSamplingRate").and_then(|s| s.parse::<u32>().ok());

            let base_url = base_url_re
                .captures(inner_content)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            let stream_url = if !base_url.is_empty() {
                if base_url.starts_with("http://") || base_url.starts_with("https://") {
                    base_url
                } else {
                    format!(
                        "{}/{}",
                        mpd_url.trim_end_matches('/'),
                        base_url.trim_start_matches('/')
                    )
                }
            } else {
                mpd_url.to_string()
            };

            let is_audio = mime_type
                .as_deref()
                .map(|m| m.starts_with("audio"))
                .unwrap_or(false)
                || height.is_none() && width.is_none();

            let itag_num = id.parse::<u32>().unwrap_or(0);
            let format_id_str = format!("dash-{}", id);

            let ext = if is_audio {
                if codecs
                    .as_deref()
                    .map(|c| c.contains("opus"))
                    .unwrap_or(false)
                {
                    "opus".to_string()
                } else {
                    "m4a".to_string()
                }
            } else if codecs
                .as_deref()
                .map(|c| c.contains("av01") || c.contains("vp9"))
                .unwrap_or(false)
            {
                "webm".to_string()
            } else {
                "mp4".to_string()
            };

            formats.push(StreamFormat {
                format_id: FormatId::new(format_id_str),
                url: stream_url,
                ext,
                resolution: Resolution { width, height },
                fps: Some(30),
                bitrate: bandwidth,
                filesize: None,
                filesize_approx: None,
                media_type: if is_audio {
                    MediaType::AudioOnly
                } else {
                    MediaType::VideoOnly
                },
                protocol: Protocol::Https,
                vcodec: if is_audio {
                    None
                } else {
                    codecs.clone().or_else(|| Some("h264".to_string()))
                },
                acodec: if is_audio {
                    codecs.clone().or_else(|| Some("aac".to_string()))
                } else {
                    None
                },
                audio_sample_rate: audio_sampling_rate,
                audio_channels: if is_audio { Some(2) } else { None },
                itag: itag_num,
                quality_label: height.map(|h| format!("{}p", h)),
                source_client: "youtube_dash_mpd".to_string(),
            });
        }

        formats
    }
}

fn extract_attr(attr_str: &str, attr_name: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"{}=["']([^"']+)["']"#, attr_name)).ok()?;
    re.captures(attr_str)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}
