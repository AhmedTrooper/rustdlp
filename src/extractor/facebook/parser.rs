use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static REP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<Representation\s+([^>]+)>([\s\S]*?)</Representation>"#).unwrap()
});

static ATTR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(\w+)="([^"]*)""#).unwrap());

static BASE_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<BaseURL>([^<]+)</BaseURL>"#).unwrap());

#[derive(Debug, Default)]
pub struct FacebookParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct FacebookParser;

impl FacebookParser {
    pub fn parse_webpage(html: &str, video_id: &str) -> FacebookParsedData {
        let raw_title = Self::extract_meta(html, "og:title")
            .or_else(|| Self::extract_meta(html, "twitter:title"))
            .or_else(|| Self::extract_title_tag(html));

        let description = Self::extract_meta(html, "og:description")
            .or_else(|| Self::extract_meta(html, "description"));

        let mut thumbnails = Vec::new();
        if let Some(thumb) = Self::extract_meta(html, "og:image") {
            thumbnails.push(thumb);
        }

        // Clean title & uploader if in format: "views · reactions | Title | Uploader"
        let (title, uploader) = if let Some(t) = raw_title {
            let parts: Vec<&str> = t.split('|').collect();
            if parts.len() >= 3 {
                (
                    Some(parts[1].trim().to_string()),
                    Some(parts[2].trim().to_string()),
                )
            } else if parts.len() == 2 {
                (
                    Some(parts[0].trim().to_string()),
                    Some(parts[1].trim().to_string()),
                )
            } else {
                (Some(t), None)
            }
        } else {
            (Some(format!("Facebook video #{}", video_id)), None)
        };

        // 2. Extract scripts and walk JSON trees
        let script_re = Regex::new(r#"<script[^>]*>([\s\S]*?)</script>"#).unwrap();
        let mut found_dash_manifests = Vec::new();
        let mut hd_urls = Vec::new();
        let mut sd_urls = Vec::new();

        for cap in script_re.captures_iter(html) {
            let script_content = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if script_content.starts_with('{')
                && script_content.ends_with('}')
                && let Ok(val) = serde_json::from_str::<Value>(script_content)
            {
                Self::walk_json_tree(&val, &mut found_dash_manifests, &mut hd_urls, &mut sd_urls);
            }
        }

        let mut formats = Vec::new();

        // 3. Parse DASH manifests
        for manifest_xml in found_dash_manifests {
            let dash_formats = Self::parse_dash_manifest(&manifest_xml);
            formats.extend(dash_formats);
        }

        // 4. Add Progressive HD format if found
        for (i, hd_url) in hd_urls.into_iter().enumerate() {
            let format_id = if i == 0 {
                "hd".to_string()
            } else {
                format!("hd-{}", i + 1)
            };
            formats.push(StreamFormat {
                format_id: FormatId::new(format_id),
                url: hd_url,
                ext: "mp4".to_string(),
                resolution: Resolution {
                    width: Some(1280),
                    height: Some(720),
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
                itag: 720,
                quality_label: Some("HD".to_string()),
                source_client: "facebook_progressive".to_string(),
            });
        }

        // 5. Add Progressive SD format if found
        for (i, sd_url) in sd_urls.into_iter().enumerate() {
            let format_id = if i == 0 {
                "sd".to_string()
            } else {
                format!("sd-{}", i + 1)
            };
            formats.push(StreamFormat {
                format_id: FormatId::new(format_id),
                url: sd_url,
                ext: "mp4".to_string(),
                resolution: Resolution {
                    width: Some(640),
                    height: Some(360),
                },
                fps: Some(30),
                bitrate: Some(800_000),
                filesize: None,
                filesize_approx: None,
                media_type: MediaType::Combined,
                protocol: Protocol::Https,
                vcodec: Some("h264".to_string()),
                acodec: Some("aac".to_string()),
                audio_sample_rate: Some(44100),
                audio_channels: Some(2),
                itag: 360,
                quality_label: Some("SD".to_string()),
                source_client: "facebook_progressive".to_string(),
            });
        }

        FacebookParsedData {
            title,
            description,
            uploader,
            duration: None,
            thumbnails,
            formats,
        }
    }

    fn walk_json_tree(
        val: &Value,
        dash_manifests: &mut Vec<String>,
        hd_urls: &mut Vec<String>,
        sd_urls: &mut Vec<String>,
    ) {
        match val {
            Value::Object(map) => {
                for (k, v) in map {
                    if matches!(
                        k.as_str(),
                        "dash_manifest_xml_string" | "dash_manifest" | "manifest_xml"
                    ) && let Some(xml_str) = v.as_str()
                        && (xml_str.contains("<MPD") || xml_str.contains("<?xml"))
                    {
                        dash_manifests.push(xml_str.to_string());
                    } else if matches!(
                        k.as_str(),
                        "browser_native_hd_url" | "playable_url_quality_hd" | "hd_src"
                    ) && let Some(url_str) = v.as_str()
                        && url_str.starts_with("http")
                        && !hd_urls.contains(&url_str.to_string())
                    {
                        hd_urls.push(url_str.to_string());
                    } else if matches!(
                        k.as_str(),
                        "browser_native_sd_url" | "playable_url" | "sd_src"
                    ) && let Some(url_str) = v.as_str()
                        && url_str.starts_with("http")
                        && !sd_urls.contains(&url_str.to_string())
                    {
                        sd_urls.push(url_str.to_string());
                    } else {
                        Self::walk_json_tree(v, dash_manifests, hd_urls, sd_urls);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::walk_json_tree(item, dash_manifests, hd_urls, sd_urls);
                }
            }
            _ => {}
        }
    }

    pub fn parse_dash_manifest(xml: &str) -> Vec<StreamFormat> {
        let mut formats = Vec::new();

        for cap in REP_REGEX.captures_iter(xml) {
            let attr_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let body_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let mut id = String::new();
            let mut mime = String::new();
            let mut width: Option<u32> = None;
            let mut height: Option<u32> = None;
            let mut bandwidth: Option<u64> = None;
            let mut codecs: Option<String> = None;
            let mut quality_label: Option<String> = None;

            for attr in ATTR_REGEX.captures_iter(attr_str) {
                let key = attr.get(1).map(|m| m.as_str()).unwrap_or("");
                let val = attr.get(2).map(|m| m.as_str()).unwrap_or("");
                match key {
                    "id" => id = val.to_string(),
                    "mimeType" => mime = val.to_string(),
                    "width" => width = val.parse().ok(),
                    "height" => height = val.parse().ok(),
                    "bandwidth" => bandwidth = val.parse().ok(),
                    "codecs" => codecs = Some(val.to_string()),
                    "FBQualityLabel" => quality_label = Some(val.to_string()),
                    _ => {}
                }
            }

            let base_url = BASE_URL_REGEX
                .captures(body_str)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());

            if let Some(stream_url) = base_url {
                let is_audio = mime.contains("audio");
                let is_video = mime.contains("video");

                let media_type = if is_audio {
                    MediaType::AudioOnly
                } else if is_video {
                    MediaType::VideoOnly
                } else {
                    MediaType::Combined
                };

                let ext = if is_audio {
                    "m4a".to_string()
                } else {
                    "mp4".to_string()
                };

                let vcodec = if is_video { codecs.clone() } else { None };
                let acodec = if is_audio { codecs.clone() } else { None };

                let itag = height.unwrap_or(if is_audio { 140 } else { 0 });

                formats.push(StreamFormat {
                    format_id: FormatId::new(if id.is_empty() {
                        format!("dash-{}", itag)
                    } else {
                        id
                    }),
                    url: stream_url,
                    ext,
                    resolution: Resolution { width, height },
                    fps: Some(30),
                    bitrate: bandwidth,
                    filesize: None,
                    filesize_approx: None,
                    media_type,
                    protocol: Protocol::Https,
                    vcodec,
                    acodec,
                    audio_sample_rate: if is_audio { Some(48000) } else { None },
                    audio_channels: if is_audio { Some(2) } else { None },
                    itag,
                    quality_label,
                    source_client: "facebook_dash".to_string(),
                });
            }
        }

        formats
    }

    fn extract_meta(html: &str, name_or_prop: &str) -> Option<String> {
        let pattern = format!(
            r#"<meta\s+(?:property|name)=["']{}["']\s+content=["'](.*?)["']"#,
            name_or_prop
        );
        let re = Regex::new(&pattern).ok()?;
        re.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| Self::clean_html_entities(m.as_str()))
    }

    fn extract_title_tag(html: &str) -> Option<String> {
        let re = Regex::new(r#"<title>(.*?)</title>"#).ok()?;
        re.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| Self::clean_html_entities(m.as_str()))
    }

    fn clean_html_entities(raw: &str) -> String {
        raw.replace("&amp;", "&")
            .replace("&#039;", "'")
            .replace("&#39;", "'")
            .replace("&quot;", "\"")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&#xb7;", "·")
            .replace("&#183;", "·")
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dash_manifest() {
        let xml = r#"
        <MPD xmlns="urn:mpeg:dash:schema:mpd:2011">
            <Period>
                <AdaptationSet>
                    <Representation id="1080p_v" mimeType="video/mp4" width="1920" height="1080" bandwidth="4500000" codecs="avc1.640028">
                        <BaseURL>https://video.fbcdn.net/1080p.mp4</BaseURL>
                    </Representation>
                    <Representation id="audio_128" mimeType="audio/mp4" bandwidth="128000" codecs="mp4a.40.2">
                        <BaseURL>https://video.fbcdn.net/audio.m4a</BaseURL>
                    </Representation>
                </AdaptationSet>
            </Period>
        </MPD>
        "#;

        let formats = FacebookParser::parse_dash_manifest(xml);
        assert_eq!(formats.len(), 2);

        let video = formats.iter().find(|f| f.is_video()).unwrap();
        assert_eq!(video.height(), 1080);
        assert_eq!(video.format_id.as_str(), "1080p_v");
        assert_eq!(video.url, "https://video.fbcdn.net/1080p.mp4");

        let audio = formats.iter().find(|f| f.is_audio()).unwrap();
        assert_eq!(audio.format_id.as_str(), "audio_128");
        assert_eq!(audio.ext, "m4a");
        assert_eq!(audio.url, "https://video.fbcdn.net/audio.m4a");
    }
}
