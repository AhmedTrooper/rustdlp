use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use std::sync::LazyLock;

static PERIOD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<Period(?:\s+[^>]*?)?>(.*?)</Period>"#).unwrap());

static ADAPTATION_SET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<AdaptationSet\s+([^>]+)>(.*?)</AdaptationSet>"#).unwrap());

static REPRESENTATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<Representation\s+([^>]+)>(.*?)</Representation>"#).unwrap()
});

static BASE_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<BaseURL[^>]*>(.*?)</BaseURL>"#).unwrap());

static SEGMENT_TEMPLATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<SegmentTemplate\s+([^>]+)>"#).unwrap());

static AUDIO_CHANNEL_CONFIG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<AudioChannelConfiguration[^>]+value=["'](\d+)["']"#).unwrap());

pub struct YoutubeDashMpdParser;

impl YoutubeDashMpdParser {
    pub fn parse(xml: &str, mpd_url: &str) -> Vec<StreamFormat> {
        let mut formats = Vec::new();

        // 1. Iterate through Periods (or whole document if no Period tag)
        let periods: Vec<&str> = if PERIOD_RE.is_match(xml) {
            PERIOD_RE
                .captures_iter(xml)
                .filter_map(|c| c.get(1))
                .map(|m| m.as_str())
                .collect()
        } else {
            vec![xml]
        };

        for period_content in periods {
            // 2. Iterate through AdaptationSets
            for adapt_cap in ADAPTATION_SET_RE.captures_iter(period_content) {
                let adapt_attr = adapt_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let adapt_content = adapt_cap.get(2).map(|m| m.as_str()).unwrap_or("");

                // Inherited AdaptationSet attributes
                let parent_mime = extract_attr(adapt_attr, "mimeType");
                let parent_codecs = extract_attr(adapt_attr, "codecs");
                let parent_width =
                    extract_attr(adapt_attr, "width").and_then(|s| s.parse::<u32>().ok());
                let parent_height =
                    extract_attr(adapt_attr, "height").and_then(|s| s.parse::<u32>().ok());
                let parent_fps = extract_attr(adapt_attr, "frameRate").and_then(parse_fps);
                let parent_audio_rate = extract_attr(adapt_attr, "audioSamplingRate")
                    .and_then(|s| s.parse::<u32>().ok());
                let parent_audio_channels = AUDIO_CHANNEL_CONFIG_RE
                    .captures(adapt_content)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<u32>().ok());

                // 3. Iterate through Representations
                for rep_cap in REPRESENTATION_RE.captures_iter(adapt_content) {
                    let rep_attr = rep_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    let rep_content = rep_cap.get(2).map(|m| m.as_str()).unwrap_or("");

                    let id = extract_attr(rep_attr, "id").unwrap_or_else(|| "0".to_string());
                    let width = extract_attr(rep_attr, "width")
                        .and_then(|s| s.parse::<u32>().ok())
                        .or(parent_width);
                    let height = extract_attr(rep_attr, "height")
                        .and_then(|s| s.parse::<u32>().ok())
                        .or(parent_height);
                    let bandwidth =
                        extract_attr(rep_attr, "bandwidth").and_then(|s| s.parse::<u64>().ok());
                    let fps = extract_attr(rep_attr, "frameRate")
                        .and_then(parse_fps)
                        .or(parent_fps);
                    let mime_type = extract_attr(rep_attr, "mimeType").or(parent_mime.clone());
                    let codecs = extract_attr(rep_attr, "codecs").or(parent_codecs.clone());
                    let audio_sampling_rate = extract_attr(rep_attr, "audioSamplingRate")
                        .and_then(|s| s.parse::<u32>().ok())
                        .or(parent_audio_rate);
                    let audio_channels = AUDIO_CHANNEL_CONFIG_RE
                        .captures(rep_content)
                        .and_then(|c| c.get(1))
                        .and_then(|m| m.as_str().parse::<u32>().ok())
                        .or(parent_audio_channels);

                    // BaseURL resolution
                    let base_url = BASE_URL_RE
                        .captures(rep_content)
                        .or_else(|| BASE_URL_RE.captures(adapt_content))
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
                        // Check SegmentTemplate initialization URL
                        if let Some(st_cap) = SEGMENT_TEMPLATE_RE
                            .captures(rep_content)
                            .or_else(|| SEGMENT_TEMPLATE_RE.captures(adapt_content))
                            && let Some(st_attr) = st_cap.get(1)
                            && let Some(media) = extract_attr(st_attr.as_str(), "media")
                                .or_else(|| extract_attr(st_attr.as_str(), "initialization"))
                        {
                            let resolved = media.replace("$RepresentationID$", &id);
                            format!(
                                "{}/{}",
                                mpd_url.trim_end_matches('/'),
                                resolved.trim_start_matches('/')
                            )
                        } else {
                            mpd_url.to_string()
                        }
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
                        fps,
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
                        audio_channels,
                        itag: itag_num,
                        quality_label: height.map(|h| format!("{}p", h)),
                        source_client: "youtube_dash_mpd".to_string(),
                    });
                }
            }
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

fn parse_fps(fps_str: String) -> Option<u32> {
    if let Some((num, den)) = fps_str.split_once('/') {
        let n = num.parse::<f64>().ok()?;
        let d = den.parse::<f64>().ok()?;
        if d > 0.0 {
            return Some((n / d).round() as u32);
        }
    }
    fps_str
        .parse::<u32>()
        .ok()
        .or_else(|| fps_str.parse::<f64>().ok().map(|f| f.round() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nested_dash_mpd() {
        let sample_mpd = r#"<?xml version="1.0" encoding="UTF-8"?>
        <MPD xmlns="urn:mpeg:dash:schema:mpd:2011" minBufferTime="PT1.5S" type="static">
            <Period>
                <AdaptationSet mimeType="video/mp4" codecs="avc1.640028" frameRate="24000/1001" width="1920" height="1080">
                    <Representation id="137" bandwidth="3123456">
                        <BaseURL>https://example.com/video137.mp4</BaseURL>
                    </Representation>
                </AdaptationSet>
                <AdaptationSet mimeType="audio/mp4" codecs="mp4a.40.2" audioSamplingRate="44100">
                    <AudioChannelConfiguration value="2"/>
                    <Representation id="140" bandwidth="129000">
                        <BaseURL>https://example.com/audio140.m4a</BaseURL>
                    </Representation>
                </AdaptationSet>
            </Period>
        </MPD>"#;

        let formats = YoutubeDashMpdParser::parse(sample_mpd, "https://example.com/manifest.mpd");
        assert_eq!(formats.len(), 2);
        let video = formats.iter().find(|f| f.is_video_only()).unwrap();
        assert_eq!(video.resolution.height, Some(1080));
        assert_eq!(video.fps, Some(24));
        assert_eq!(video.vcodec.as_deref(), Some("avc1.640028"));

        let audio = formats.iter().find(|f| f.is_audio_only()).unwrap();
        assert_eq!(audio.audio_channels, Some(2));
        assert_eq!(audio.audio_sample_rate, Some(44100));
    }
}
