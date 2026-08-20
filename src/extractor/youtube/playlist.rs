use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::models::playlist::{PlaylistItem, PlaylistMetadata};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, USER_AGENT};
use serde_json::{Value, json};
use std::collections::HashSet;

pub struct YoutubePlaylistExtractor {
    http: reqwest::Client,
}

impl YoutubePlaylistExtractor {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn extract_playlist(&self, playlist_id: &str) -> Result<PlaylistMetadata> {
        let url = "https://www.youtube.com/youtubei/v1/browse?prettyPrint=false";
        let browse_id = if playlist_id.starts_with("VL") {
            playlist_id.to_string()
        } else {
            format!("VL{}", playlist_id)
        };

        let payload = json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": "2.20260708.00.00",
                    "hl": "en",
                    "timeZone": "UTC",
                    "utcOffsetMinutes": 0
                }
            },
            "browseId": browse_id
        });

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
            ),
        );
        headers.insert("X-YouTube-Client-Name", HeaderValue::from_static("1"));
        headers.insert(
            "X-YouTube-Client-Version",
            HeaderValue::from_static("2.20260708.00.00"),
        );
        headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));

        let response = self
            .http
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Playlist API request failed with HTTP {}",
                response.status()
            )));
        }

        let json_val: Value = response.json().await?;

        let title = json_val
            .pointer("/metadata/playlistMetadataRenderer/title")
            .and_then(|v| v.as_str())
            .or_else(|| {
                json_val
                    .pointer("/header/playlistHeaderRenderer/title/simpleText")
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("Unknown Playlist")
            .to_string();

        let description = json_val
            .pointer("/metadata/playlistMetadataRenderer/description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut items = Vec::new();
        let mut seen_ids = HashSet::new();

        Self::extract_videos_from_value(&json_val, &mut items, &mut seen_ids);

        let webpage_url = format!("https://www.youtube.com/playlist?list={}", playlist_id);

        Ok(PlaylistMetadata {
            id: playlist_id.to_string(),
            title,
            description,
            uploader: None,
            items,
            webpage_url,
        })
    }

    fn extract_videos_from_value(
        val: &Value,
        items: &mut Vec<PlaylistItem>,
        seen_ids: &mut HashSet<String>,
    ) {
        match val {
            Value::Object(map) => {
                if let Some(vid_val) = map.get("videoId").and_then(|v| v.as_str())
                    && vid_val.len() == 11
                    && !seen_ids.contains(vid_val)
                {
                    seen_ids.insert(vid_val.to_string());

                    let title = map
                        .get("title")
                        .and_then(|t| {
                            t.get("simpleText")
                                .and_then(|s| s.as_str())
                                .or_else(|| t.pointer("/runs/0/text").and_then(|s| s.as_str()))
                        })
                        .map(|s| s.to_string());

                    let duration = map
                        .get("lengthSeconds")
                        .and_then(|s| s.as_str())
                        .and_then(|s| s.parse::<u64>().ok());

                    items.push(PlaylistItem {
                        id: VideoId::new(vid_val),
                        title,
                        uploader: None,
                        duration,
                        thumbnail: None,
                    });
                }

                for v in map.values() {
                    Self::extract_videos_from_value(v, items, seen_ids);
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::extract_videos_from_value(item, items, seen_ids);
                }
            }
            _ => {}
        }
    }
}
