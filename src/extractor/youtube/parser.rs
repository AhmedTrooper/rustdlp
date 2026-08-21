use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::extractor::youtube::cipher::CipherHelper;
use crate::extractor::youtube::jsc::JsChallengeSolver;
use crate::models::format::StreamFormat;
use crate::models::innertube::RawFormatStream;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static MIME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^([^/]+)/([^;]+)(?:;\s*codecs="([^"]+)")?"#).unwrap());

pub struct YoutubeParser;

impl YoutubeParser {
    pub fn parse_player_response(
        val: &Value,
        source_ua: Option<&str>,
        solver: Option<&JsChallengeSolver>,
    ) -> Vec<StreamFormat> {
        let mut formats = Vec::new();
        let streaming_data = val.get("streamingData").unwrap_or(val);
        let client_tag = source_ua.unwrap_or("innertube");

        if let Some(arr) = streaming_data.get("formats").and_then(|v| v.as_array()) {
            for item in arr {
                if let Ok(raw) = serde_json::from_value::<RawFormatStream>(item.clone())
                    && let Some(f) = parse_stream_format(&raw, client_tag, solver)
                {
                    formats.push(f);
                }
            }
        }

        if let Some(arr) = streaming_data
            .get("adaptiveFormats")
            .and_then(|v| v.as_array())
        {
            for item in arr {
                if let Ok(raw) = serde_json::from_value::<RawFormatStream>(item.clone())
                    && let Some(f) = parse_stream_format(&raw, client_tag, solver)
                {
                    formats.push(f);
                }
            }
        }

        formats
    }
}

pub fn parse_stream_format(
    raw: &RawFormatStream,
    client_name: &str,
    solver: Option<&JsChallengeSolver>,
) -> Option<StreamFormat> {
    let raw_url = if let Some(ref u) = raw.url {
        Some(u.clone())
    } else if let Some(cipher) = raw.signature_cipher.as_ref().or(raw.cipher.as_ref()) {
        CipherHelper::parse_signature_cipher(cipher, solver).ok()
    } else {
        None
    }?;

    let itag = raw.itag;
    let format_id_str = itag.to_string();

    let (media_type, ext, vcodec, acodec) = if let Some(ref mime) = raw.mime_type {
        parse_mime_type(mime)
    } else {
        (MediaType::Combined, "mp4".to_string(), None, None)
    };

    let resolution = Resolution {
        width: raw.width,
        height: raw.height,
    };

    Some(StreamFormat {
        itag,
        format_id: FormatId::new(format_id_str),
        url: raw_url,
        ext,
        resolution,
        fps: raw.fps,
        vcodec,
        acodec,
        bitrate: raw.bitrate,
        filesize: raw.content_length.as_deref().and_then(|s| s.parse().ok()),
        filesize_approx: raw.approx_duration_ms.as_deref().and_then(|ms| {
            let dur_sec = ms.parse::<f64>().ok()? / 1000.0;
            let br = raw.bitrate? as f64;
            Some(((br * dur_sec) / 8.0) as u64)
        }),
        media_type,
        protocol: Protocol::Https,
        quality_label: raw.quality_label.clone(),
        audio_channels: raw.audio_channels,
        audio_sample_rate: raw
            .audio_sample_rate
            .as_deref()
            .and_then(|s| s.parse().ok()),
        source_client: client_name.to_string(),
    })
}

fn parse_mime_type(mime: &str) -> (MediaType, String, Option<String>, Option<String>) {
    if let Some(caps) = MIME_REGEX.captures(mime) {
        let type_major = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let subtype = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let codecs = caps.get(3).map(|m| m.as_str().to_string());

        let media_type = match type_major {
            "audio" => MediaType::AudioOnly,
            "video" => {
                if let Some(ref c) = codecs {
                    if c.contains(',') {
                        MediaType::Combined
                    } else {
                        MediaType::VideoOnly
                    }
                } else {
                    MediaType::Combined
                }
            }
            _ => MediaType::Combined,
        };

        let ext = match subtype {
            "mp4" => {
                if media_type == MediaType::AudioOnly {
                    "m4a".to_string()
                } else {
                    "mp4".to_string()
                }
            }
            "webm" => {
                if media_type == MediaType::AudioOnly {
                    "opus".to_string()
                } else {
                    "webm".to_string()
                }
            }
            "3gpp" => "3gp".to_string(),
            other => other.to_string(),
        };

        let mut vcodec = None;
        let mut acodec = None;

        if let Some(ref c) = codecs {
            if c.contains(',') {
                let parts: Vec<&str> = c.split(',').map(|s| s.trim()).collect();
                vcodec = parts.first().map(|s| s.to_string());
                acodec = parts.get(1).map(|s| s.to_string());
            } else if media_type == MediaType::AudioOnly {
                acodec = Some(c.clone());
            } else {
                vcodec = Some(c.clone());
            }
        }

        (media_type, ext, vcodec, acodec)
    } else {
        (MediaType::Combined, "mp4".to_string(), None, None)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_parse_mime() {
        let mime = r#"video/mp4; codecs="avc1.640028, mp4a.40.2""#;
        let (media_type, ext, vcodec, acodec) = parse_mime_type(mime);
        assert_eq!(media_type, MediaType::Combined);
        assert_eq!(ext, "mp4");
        assert_eq!(vcodec.as_deref(), Some("avc1.640028"));
        assert_eq!(acodec.as_deref(), Some("mp4a.40.2"));
    }
}
