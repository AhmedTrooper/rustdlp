use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct TikTokParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub view_count: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct TikTokParser;

impl TikTokParser {
    pub fn parse_html(html: &str, video_id: &str) -> TikTokParsedData {
        let mut data = TikTokParsedData::default();

        // 1. Search for JSON hydrate script
        let json_scripts = [
            r#"<script\s+id="__UNIVERSAL_DATA_FOR_REHYDRATION__"[^>]*>([\s\S]*?)</script>"#,
            r#"<script\s+id="SIGI_STATE"[^>]*>([\s\S]*?)</script>"#,
            r#"<script\s+id="__NEXT_DATA__"[^>]*>([\s\S]*?)</script>"#,
        ];

        for pattern in json_scripts {
            if let Ok(re) = Regex::new(pattern)
                && let Some(cap) = re.captures(html)
                && let Some(script_content) = cap.get(1)
                && let Ok(val) = serde_json::from_str::<Value>(script_content.as_str())
            {
                Self::extract_from_json(&val, &mut data, video_id);
                if !data.formats.is_empty() {
                    break;
                }
            }
        }

        // 2. OpenGraph fallback
        if data.title.is_none() {
            let og_title_re =
                Regex::new(r#"<meta\s+(?:property|name)=["']og:title["']\s+content=["'](.*?)["']"#)
                    .unwrap();
            data.title = og_title_re
                .captures(html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
        }

        if data.description.is_none() {
            let og_desc_re = Regex::new(
                r#"<meta\s+(?:property|name)=["']og:description["']\s+content=["'](.*?)["']"#,
            )
            .unwrap();
            data.description = og_desc_re
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
            data.title = Some(format!("TikTok video #{}", video_id));
        }

        data
    }

    fn extract_from_json(val: &Value, data: &mut TikTokParsedData, target_id: &str) {
        // Look for ItemStruct or videoDetail in JSON tree
        let item = val
            .pointer("/__DEFAULT_SCOPE__/webapp.videoDetail/itemInfo/itemStruct")
            .or_else(|| val.pointer(&format!("/ItemModule/{}/itemStruct", target_id)))
            .or_else(|| val.pointer("/itemInfo/itemStruct"))
            .unwrap_or(val);

        if let Some(desc) = item.get("desc").and_then(|v| v.as_str()) {
            data.title = Some(desc.to_string());
            data.description = Some(desc.to_string());
        }

        if let Some(author) = item.get("author") {
            data.uploader = author
                .get("nickname")
                .or_else(|| author.get("uniqueId"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        if let Some(video) = item.get("video") {
            let duration = video.get("duration").and_then(|v| v.as_u64());
            data.duration = duration;

            let width = video
                .get("width")
                .and_then(|v| v.as_u64())
                .map(|w| w as u32);
            let height = video
                .get("height")
                .and_then(|v| v.as_u64())
                .map(|h| h as u32);

            if let Some(cover) = video.get("cover").and_then(|v| v.as_str()) {
                data.thumbnails.push(cover.to_string());
            }

            // Direct playAddr (HD no watermark)
            if let Some(play_addr) = video.get("playAddr").and_then(|v| v.as_str()) {
                data.formats.push(StreamFormat {
                    format_id: FormatId::new("hd"),
                    url: play_addr.to_string(),
                    ext: "mp4".to_string(),
                    resolution: Resolution { width, height },
                    fps: Some(30),
                    bitrate: video.get("bitrate").and_then(|v| v.as_u64()),
                    filesize: None,
                    filesize_approx: None,
                    media_type: MediaType::Combined,
                    protocol: Protocol::Https,
                    vcodec: Some("h264".to_string()),
                    acodec: Some("aac".to_string()),
                    audio_sample_rate: Some(44100),
                    audio_channels: Some(2),
                    itag: height.unwrap_or(1080),
                    quality_label: Some("HD No Watermark".to_string()),
                    source_client: "tiktok_direct".to_string(),
                });
            }

            // downloadAddr
            if let Some(download_addr) = video.get("downloadAddr").and_then(|v| v.as_str()) {
                data.formats.push(StreamFormat {
                    format_id: FormatId::new("watermarked"),
                    url: download_addr.to_string(),
                    ext: "mp4".to_string(),
                    resolution: Resolution { width, height },
                    fps: Some(30),
                    bitrate: video.get("bitrate").and_then(|v| v.as_u64()),
                    filesize: None,
                    filesize_approx: None,
                    media_type: MediaType::Combined,
                    protocol: Protocol::Https,
                    vcodec: Some("h264".to_string()),
                    acodec: Some("aac".to_string()),
                    audio_sample_rate: Some(44100),
                    audio_channels: Some(2),
                    itag: height.unwrap_or(720),
                    quality_label: Some("Watermarked".to_string()),
                    source_client: "tiktok_download".to_string(),
                });
            }

            // bitrateInfo multi-qualities
            if let Some(bitrate_info) = video.get("bitrateInfo").and_then(|v| v.as_array()) {
                for (idx, b) in bitrate_info.iter().enumerate() {
                    let gear = b
                        .get("GearName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("standard");
                    let bitrate = b.get("Bitrate").and_then(|v| v.as_u64());
                    let play_url = b.pointer("/PlayAddr/UrlList/0").and_then(|v| v.as_str());

                    if let Some(u) = play_url {
                        data.formats.push(StreamFormat {
                            format_id: FormatId::new(format!("{}-{}", gear, idx)),
                            url: u.to_string(),
                            ext: "mp4".to_string(),
                            resolution: Resolution { width, height },
                            fps: Some(30),
                            bitrate,
                            filesize: None,
                            filesize_approx: None,
                            media_type: MediaType::Combined,
                            protocol: Protocol::Https,
                            vcodec: Some("h264".to_string()),
                            acodec: Some("aac".to_string()),
                            audio_sample_rate: Some(44100),
                            audio_channels: Some(2),
                            itag: (idx + 1) as u32,
                            quality_label: Some(gear.to_string()),
                            source_client: "tiktok_bitrate_info".to_string(),
                        });
                    }
                }
            }
        }

        // Original Audio stream
        if let Some(music) = item.get("music")
            && let Some(play_url) = music.get("playUrl").and_then(|v| v.as_str())
        {
            data.formats.push(StreamFormat {
                format_id: FormatId::new("audio-original"),
                url: play_url.to_string(),
                ext: "mp3".to_string(),
                resolution: Resolution::default(),
                fps: None,
                bitrate: Some(128_000),
                filesize: None,
                filesize_approx: None,
                media_type: MediaType::AudioOnly,
                protocol: Protocol::Https,
                vcodec: None,
                acodec: Some("mp3".to_string()),
                audio_sample_rate: Some(44100),
                audio_channels: Some(2),
                itag: 140,
                quality_label: Some("Original Audio".to_string()),
                source_client: "tiktok_audio".to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiktok_parse() {
        let html = r#"
        <script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">
        {
            "__DEFAULT_SCOPE__": {
                "webapp.videoDetail": {
                    "itemInfo": {
                        "itemStruct": {
                            "id": "7106594312292453678",
                            "desc": "Check out this amazing TikTok dance video #viral #dance",
                            "author": { "uniqueId": "dancer", "nickname": "Top Dancer" },
                            "video": {
                                "duration": 15,
                                "width": 1080,
                                "height": 1920,
                                "playAddr": "https://v16-webapp-prime.tiktok.com/video/play.mp4",
                                "bitrate": 2500000
                            },
                            "music": { "playUrl": "https://sf16-ies-music.tiktokcdn.com/music.mp3" }
                        }
                    }
                }
            }
        }
        </script>
        "#;

        let data = TikTokParser::parse_html(html, "7106594312292453678");
        assert_eq!(
            data.title.as_deref(),
            Some("Check out this amazing TikTok dance video #viral #dance")
        );
        assert_eq!(data.uploader.as_deref(), Some("Top Dancer"));
        assert_eq!(data.formats.len(), 2);
    }
}
