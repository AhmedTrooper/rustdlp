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
        solver: Option<&JsChallengeSolver>,
    ) -> Vec<StreamFormat> {
        let mut formats = Vec::new();
        let streaming_data = val.get("streamingData").unwrap_or(val);

        if let Some(arr) = streaming_data.get("formats").and_then(|v| v.as_array()) {
            for item in arr {
                if let Ok(raw) = serde_json::from_value::<RawFormatStream>(item.clone())
                    && let Some(f) = parse_stream_format(&raw, "innertube", solver)
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
                    && let Some(f) = parse_stream_format(&raw, "innertube_dash", solver)
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

    let mime = raw.mime_type.as_deref().unwrap_or("");
    let (container, vcodec, acodec) = parse_mime_and_codecs(mime);

    let media_type = match (vcodec.is_some(), acodec.is_some()) {
        (true, true) => MediaType::Combined,
        (true, false) => MediaType::VideoOnly,
        (false, true) => MediaType::AudioOnly,
        (false, false) => {
            if mime.starts_with("video") {
                MediaType::VideoOnly
            } else if mime.starts_with("audio") {
                MediaType::AudioOnly
            } else {
                MediaType::Combined
            }
        }
    };

    let ext = match (media_type, container.as_str()) {
        (MediaType::AudioOnly, "mp4") => "m4a".to_string(),
        (MediaType::AudioOnly, "webm") if acodec.as_deref() == Some("opus") => "opus".to_string(),
        (_, other) => other.to_string(),
    };

    let width = raw.width;
    let height = raw
        .height
        .or_else(|| parse_height_from_quality_label(raw.quality_label.as_deref()));

    let resolution = Resolution::new(width, height);
    let fps = raw.fps.filter(|&f| f > 1);

    let filesize = raw
        .content_length
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok());

    let bitrate = raw.average_bitrate.or(raw.bitrate);

    let approx_duration_sec = raw
        .approx_duration_ms
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|ms| ms / 1000);

    let filesize_approx = if filesize.is_none() {
        if let (Some(br), Some(dur)) = (bitrate, approx_duration_sec) {
            Some((br * dur) / 8)
        } else {
            None
        }
    } else {
        None
    };

    let audio_sample_rate = raw
        .audio_sample_rate
        .as_deref()
        .and_then(|s| s.parse::<u32>().ok());

    let quality_label = raw.quality_label.clone().or_else(|| raw.quality.clone());

    Some(StreamFormat {
        itag: raw.itag,
        format_id: FormatId::new(raw.itag.to_string()),
        url: raw_url,
        ext,
        resolution,
        fps,
        vcodec,
        acodec,
        bitrate,
        filesize,
        filesize_approx,
        media_type,
        protocol: Protocol::Https,
        quality_label,
        audio_channels: raw.audio_channels,
        audio_sample_rate,
        source_client: client_name.to_string(),
    })
}

fn parse_mime_and_codecs(mime: &str) -> (String, Option<String>, Option<String>) {
    if let Some(caps) = MIME_REGEX.captures(mime) {
        let major = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let sub = caps.get(2).map(|m| m.as_str()).unwrap_or("mp4");
        let raw_codecs = caps.get(3).map(|m| m.as_str());

        let mut vcodec = None;
        let mut acodec = None;

        if let Some(codecs_str) = raw_codecs {
            let codecs: Vec<&str> = codecs_str.split(',').map(|s| s.trim()).collect();
            for codec in codecs {
                if codec.starts_with("avc")
                    || codec.starts_with("vp")
                    || codec.starts_with("av01")
                    || codec.starts_with("hev")
                {
                    vcodec = Some(codec.to_string());
                } else if codec.starts_with("mp4a")
                    || codec.starts_with("opus")
                    || codec.starts_with("vorbis")
                    || codec.starts_with("ac-3")
                {
                    acodec = Some(codec.to_string());
                }
            }
        }

        if vcodec.is_none() && major == "video" {
            vcodec = Some(sub.to_string());
        }
        if acodec.is_none() && major == "audio" {
            acodec = Some(sub.to_string());
        }

        (sub.to_string(), vcodec, acodec)
    } else {
        ("mp4".to_string(), None, None)
    }
}

fn parse_height_from_quality_label(label: Option<&str>) -> Option<u32> {
    let l = label?;
    let digits: String = l.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_parse_mime() {
        let (container, vcodec, acodec) =
            parse_mime_and_codecs(r#"video/mp4; codecs="avc1.640028, mp4a.40.2""#);
        assert_eq!(container, "mp4");
        assert_eq!(vcodec, Some("avc1.640028".into()));
        assert_eq!(acodec, Some("mp4a.40.2".into()));
    }
}
