use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct SoundCloudParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct SoundCloudParser;

impl SoundCloudParser {
    pub fn parse_html(html: &str, track_id: &str) -> SoundCloudParsedData {
        let mut data = SoundCloudParsedData::default();

        // 1. Search for window.__sc_hydration
        let hyd_re =
            Regex::new(r#"window\.__sc_hydration\s*=\s*(\[[\s\S]*?\]);\s*</script>"#).unwrap();
        if let Some(cap) = hyd_re.captures(html)
            && let Some(json_str) = cap.get(1)
            && let Ok(val) = serde_json::from_str::<Value>(json_str.as_str())
            && let Some(arr) = val.as_array()
        {
            for item in arr {
                if item.get("hydratable").and_then(|v| v.as_str()) == Some("sound")
                    && let Some(sound_data) = item.get("data")
                {
                    Self::extract_sound_data(sound_data, &mut data);
                    break;
                }
            }
        }

        // 2. OpenGraph Fallbacks
        if data.title.is_none() {
            let og_title_re =
                Regex::new(r#"<meta\s+(?:property|name)=["']og:title["']\s+content=["'](.*?)["']"#)
                    .unwrap();
            data.title = og_title_re
                .captures(html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
        }

        if data.uploader.is_none() {
            let og_user_re = Regex::new(
                r#"<meta\s+(?:property|name)=["']soundcloud:user["']\s+content=["'](.*?)["']"#,
            )
            .unwrap();
            data.uploader = og_user_re
                .captures(html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
        }

        if data.thumbnails.is_empty() {
            let og_img_re =
                Regex::new(r#"<meta\s+(?:property|name)=["']og:image["']\s+content=["'](.*?)["']"#)
                    .unwrap();
            if let Some(thumb) = og_img_re.captures(html).and_then(|c| c.get(1)) {
                data.thumbnails.push(thumb.as_str().to_string());
            }
        }

        if data.title.is_none() {
            data.title = Some(format!("SoundCloud track #{}", track_id));
        }

        data
    }

    fn extract_sound_data(val: &Value, data: &mut SoundCloudParsedData) {
        data.title = val
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        data.description = val
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        data.duration = val
            .get("duration")
            .and_then(|v| v.as_u64())
            .map(|d| d / 1000);

        if let Some(user) = val.get("user") {
            data.uploader = user
                .get("username")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        if let Some(art) = val.get("artwork_url").and_then(|v| v.as_str()) {
            data.thumbnails.push(art.replace("-large", "-t500x500"));
        }

        if let Some(media) = val.get("media")
            && let Some(transcodings) = media.get("transcodings").and_then(|v| v.as_array())
        {
            for (idx, t) in transcodings.iter().enumerate() {
                if let Some(url_str) = t.get("url").and_then(|v| v.as_str()) {
                    let mime = t
                        .pointer("/format/mime_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("audio/mpeg");
                    let protocol = t
                        .pointer("/format/protocol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("hls");

                    let is_opus = mime.contains("opus") || mime.contains("ogg");
                    let ext = if is_opus {
                        "opus".to_string()
                    } else {
                        "mp3".to_string()
                    };

                    data.formats.push(StreamFormat {
                        format_id: FormatId::new(format!("{}-{}", ext, protocol)),
                        url: url_str.to_string(),
                        ext,
                        resolution: Resolution::default(),
                        fps: None,
                        bitrate: Some(if is_opus { 64_000 } else { 128_000 }),
                        filesize: None,
                        filesize_approx: None,
                        media_type: MediaType::AudioOnly,
                        protocol: if protocol == "hls" {
                            Protocol::Hls
                        } else {
                            Protocol::Https
                        },
                        vcodec: None,
                        acodec: Some(if is_opus {
                            "opus".to_string()
                        } else {
                            "mp3".to_string()
                        }),
                        audio_sample_rate: Some(48000),
                        audio_channels: Some(2),
                        itag: (idx + 1) as u32,
                        quality_label: Some(format!(
                            "{} ({})",
                            if is_opus { "Opus" } else { "MP3" },
                            protocol
                        )),
                        source_client: "soundcloud_stream".to_string(),
                    });
                }
            }
        }
    }
}
