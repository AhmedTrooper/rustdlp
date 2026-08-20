use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use crate::models::format::StreamFormat;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct RedditParsedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
}

pub struct RedditParser;

impl RedditParser {
    pub fn parse_post_json(val: &Value, display_id: &str) -> RedditParsedData {
        let mut data = RedditParsedData::default();

        let post = val
            .pointer("/0/data/children/0/data")
            .or_else(|| val.pointer("/data/children/0/data"))
            .unwrap_or(val);

        data.title = post
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        data.uploader = post
            .get("author")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(thumb) = post.get("thumbnail").and_then(|v| v.as_str())
            && thumb.starts_with("http")
        {
            data.thumbnails.push(thumb.to_string());
        }

        let media = post
            .pointer("/secure_media/reddit_video")
            .or_else(|| post.pointer("/media/reddit_video"))
            .or_else(|| post.pointer("/crosspost_parent_list/0/secure_media/reddit_video"))
            .or_else(|| post.pointer("/crosspost_parent_list/0/media/reddit_video"));

        if let Some(vid) = media {
            let width = vid.get("width").and_then(|v| v.as_u64()).map(|w| w as u32);
            let height = vid.get("height").and_then(|v| v.as_u64()).map(|h| h as u32);
            let duration = vid.get("duration").and_then(|v| v.as_u64());
            data.duration = duration;

            let bitrate = vid
                .get("bitrate_kbps")
                .and_then(|v| v.as_u64())
                .map(|b| b * 1000);

            // 1. Fallback / direct video stream (Video Only)
            if let Some(fallback_url) = vid.get("fallback_url").and_then(|v| v.as_str()) {
                let clean_url = fallback_url
                    .split('?')
                    .next()
                    .unwrap_or(fallback_url)
                    .to_string();
                let itag = height.unwrap_or(720);

                data.formats.push(StreamFormat {
                    format_id: FormatId::new(format!("dash-video-{}", itag)),
                    url: clean_url.clone(),
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
                    itag,
                    quality_label: height.map(|h| format!("{}p", h)),
                    source_client: "reddit_video".to_string(),
                });

                // 2. Audio stream matching the v.redd.it path
                if let Some(base) = clean_url.rsplit_once('/') {
                    let audio_url = format!("{}/DASH_AUDIO_128.mp4", base.0);
                    data.formats.push(StreamFormat {
                        format_id: FormatId::new("dash-audio-128"),
                        url: audio_url,
                        ext: "m4a".to_string(),
                        resolution: Resolution::default(),
                        fps: None,
                        bitrate: Some(128_000),
                        filesize: None,
                        filesize_approx: None,
                        media_type: MediaType::AudioOnly,
                        protocol: Protocol::Https,
                        vcodec: None,
                        acodec: Some("aac".to_string()),
                        audio_sample_rate: Some(44100),
                        audio_channels: Some(2),
                        itag: 140,
                        quality_label: Some("128k audio".to_string()),
                        source_client: "reddit_audio".to_string(),
                    });
                }
            }
        }

        if data.title.is_none() {
            data.title = Some(format!("Reddit post #{}", display_id));
        }

        data
    }
}
