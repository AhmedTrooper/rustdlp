use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::extractor::traits::Extractor;
use crate::extractor::youtube::client::{InnertubeClient, InnertubeClientType};
use crate::extractor::youtube::parser::parse_stream_format;
use crate::extractor::youtube::url::extract_video_id;
use crate::models::innertube::InnertubePlayerResponse;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use std::collections::HashSet;

pub struct YoutubeExtractor {
    client: InnertubeClient,
}

impl YoutubeExtractor {
    pub fn new() -> Self {
        Self {
            client: InnertubeClient::new(),
        }
    }

    pub async fn extract_by_id(&self, video_id: &VideoId) -> Result<VideoMetadata> {
        let clients_to_try = [
            InnertubeClientType::VisionOs,
            InnertubeClientType::AndroidVr,
            InnertubeClientType::Android,
            InnertubeClientType::Ios,
        ];

        let mut responses = Vec::new();
        let mut primary_meta: Option<InnertubePlayerResponse> = None;

        for client_type in clients_to_try {
            match self.client.fetch_player(video_id, client_type).await {
                Ok(resp) => {
                    let status = resp
                        .playability_status
                        .as_ref()
                        .and_then(|s| s.status.as_deref())
                        .unwrap_or("UNKNOWN");

                    if status == "OK" {
                        if primary_meta.is_none() {
                            primary_meta = Some(resp.clone());
                        }
                        responses.push((client_type.name(), resp));
                    } else if status == "UNPLAYABLE" || status == "ERROR" {
                        let reason = resp
                            .playability_status
                            .as_ref()
                            .and_then(|s| s.reason.as_deref())
                            .unwrap_or("Video unavailable");
                        if primary_meta.is_none() {
                            // If first client returned an explicit error, store it
                            log::debug!("Client {} returned status {}: {}", client_type.name(), status, reason);
                        }
                    }
                }
                Err(e) => {
                    log::debug!("Client {} error: {}", client_type.name(), e);
                }
            }
        }

        let meta = primary_meta.ok_or_else(|| {
            DlpError::VideoUnavailable(format!(
                "Could not retrieve video stream metadata for ID: {}",
                video_id
            ))
        })?;

        let details = meta.video_details.as_ref().ok_or_else(|| {
            DlpError::ExtractionError("Missing videoDetails in player response".into())
        })?;

        let title = details.title.clone().unwrap_or_else(|| "Unknown Title".into());
        let uploader = details.author.clone().unwrap_or_else(|| "Unknown Uploader".into());
        let channel_id = details.channel_id.clone();
        let duration = details.length_seconds.as_deref().and_then(|s| s.parse::<u64>().ok());
        let view_count = details.view_count.as_deref().and_then(|s| s.parse::<u64>().ok());
        let description = details.short_description.clone();
        let is_live = details.is_live_content.unwrap_or(false);

        let thumbnails: Vec<String> = details
            .thumbnail
            .as_ref()
            .and_then(|t| t.thumbnails.as_ref())
            .map(|list| list.iter().map(|item| item.url.clone()).collect())
            .unwrap_or_default();

        let upload_date = meta
            .microformat
            .as_ref()
            .and_then(|m| m.player_microformat_renderer.as_ref())
            .and_then(|r| r.upload_date.clone().or_else(|| r.publish_date.clone()));

        // Collect and deduplicate formats across all successful client responses
        let mut all_formats = Vec::new();
        let mut seen_itags = HashSet::new();

        for (client_name, resp) in responses {
            if let Some(streaming_data) = resp.streaming_data {
                // Progressive formats (combined video + audio)
                if let Some(formats) = streaming_data.formats {
                    for raw in formats {
                        if !seen_itags.contains(&raw.itag) {
                            if let Some(stream_fmt) = parse_stream_format(&raw, client_name) {
                                seen_itags.insert(raw.itag);
                                all_formats.push(stream_fmt);
                            }
                        }
                    }
                }

                // Adaptive formats (separate video and audio streams)
                if let Some(adaptive) = streaming_data.adaptive_formats {
                    for raw in adaptive {
                        if !seen_itags.contains(&raw.itag) {
                            if let Some(stream_fmt) = parse_stream_format(&raw, client_name) {
                                seen_itags.insert(raw.itag);
                                all_formats.push(stream_fmt);
                            }
                        }
                    }
                }
            }
        }

        // Sort formats: audio first (by bitrate), then video (by resolution, fps, bitrate), then combined
        all_formats.sort_by(|a, b| {
            match (a.is_audio_only(), b.is_audio_only()) {
                (true, true) => a.effective_bitrate().cmp(&b.effective_bitrate()),
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (false, false) => {
                    let res_cmp = a.height().cmp(&b.height());
                    if res_cmp != std::cmp::Ordering::Equal {
                        res_cmp
                    } else {
                        let fps_cmp = a.effective_fps().cmp(&b.effective_fps());
                        if fps_cmp != std::cmp::Ordering::Equal {
                            fps_cmp
                        } else {
                            a.effective_bitrate().cmp(&b.effective_bitrate())
                        }
                    }
                }
            }
        });

        let webpage_url = format!("https://www.youtube.com/watch?v={}", video_id);

        Ok(VideoMetadata {
            id: video_id.clone(),
            title,
            uploader,
            channel_id,
            duration,
            view_count,
            description,
            upload_date,
            thumbnails,
            formats: all_formats,
            webpage_url,
            is_live,
        })
    }
}

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn can_extract(&self, url: &str) -> bool {
        extract_video_id(url).is_ok()
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_video_id(url)?;
        self.extract_by_id(&video_id).await
    }
}
