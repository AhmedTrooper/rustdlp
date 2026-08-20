use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct LinkedInParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct LinkedInParser;

impl LinkedInParser {
    pub fn parse_html(html: &str, video_id: &str) -> LinkedInParsedData {
        let mut data = LinkedInParsedData::default();

        // 1. Extract data-sources JSON inside <video data-sources="...">
        let sources_re = Regex::new(r#"data-sources="([^"]+)""#).unwrap();
        if let Some(cap) = sources_re.captures(html)
            && let Some(json_raw) = cap.get(1)
        {
            let clean_json = json_raw.as_str().replace("&quot;", "\"");
            if let Ok(sources) = serde_json::from_str::<Value>(&clean_json)
                && let Some(arr) = sources.as_array()
            {
                for (idx, item) in arr.iter().enumerate() {
                    if let Some(src_url) = item.get("src").and_then(|v| v.as_str()) {
                        let bitrate = item.get("data-bitrate").and_then(|v| v.as_u64());
                        let idx_num = idx + 1;
                        data.formats.push(StreamFormat {
                            format_id: FormatId::new(format!("http-{}", idx_num)),
                            url: src_url.to_string(),
                            ext: "mp4".to_string(),
                            resolution: Resolution {
                                width: Some(1280),
                                height: Some(720),
                            },
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
                            itag: idx_num as u32,
                            quality_label: Some(format!("Quality {}", idx_num)),
                            source_client: "linkedin_video".to_string(),
                        });
                    }
                }
            }
        }

        // 2. OpenGraph Fallbacks
        let og_title_re =
            Regex::new(r#"<meta\s+(?:property|name)=["']og:title["']\s+content=["'](.*?)["']"#)
                .unwrap();
        let og_desc_re = Regex::new(
            r#"<meta\s+(?:property|name)=["']og:description["']\s+content=["'](.*?)["']"#,
        )
        .unwrap();
        let og_img_re =
            Regex::new(r#"<meta\s+(?:property|name)=["']og:image["']\s+content=["'](.*?)["']"#)
                .unwrap();

        data.title = og_title_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        data.description = og_desc_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        if let Some(thumb) = og_img_re.captures(html).and_then(|c| c.get(1)) {
            data.thumbnails.push(thumb.as_str().to_string());
        }

        if data.title.is_none() {
            data.title = Some(format!("LinkedIn video #{}", video_id));
        }

        data
    }
}
