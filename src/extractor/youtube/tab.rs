use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::models::playlist::{PlaylistItem, PlaylistMetadata};
use regex::Regex;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::{Value, json};
use std::sync::LazyLock;

static YOUTUBE_PLAYLIST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:list=|\/playlist\?list=)(?P<id>(?:PL|LL|EC|UU|FL|RD|UL|TL|PU|OLAK5uy_)[0-9A-Za-z-_]{10,}|RDMM|WL|LL|LM)"#).unwrap()
});

static YOUTUBE_CHANNEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:https?://)?(?:www\.)?youtube\.com/(?:(?P<handle>@[^/?#&]+)|channel/(?P<channel_id>UC[0-9A-Za-z_-]{22})|c/(?P<custom>[^/?#&]+)|user/(?P<user>[^/?#&]+))"#).unwrap()
});

pub fn is_youtube_tab_url(url: &str) -> bool {
    YOUTUBE_PLAYLIST_RE.is_match(url) || YOUTUBE_CHANNEL_RE.is_match(url)
}

pub fn extract_playlist_id(url: &str) -> Option<String> {
    YOUTUBE_PLAYLIST_RE
        .captures(url)
        .and_then(|c| c.name("id"))
        .map(|m| m.as_str().to_string())
}

pub struct YoutubeTabExtractor {
    http: reqwest::Client,
}

impl Default for YoutubeTabExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl YoutubeTabExtractor {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_http_client(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn extract_tab(&self, url: &str) -> Result<PlaylistMetadata> {
        let playlist_id = extract_playlist_id(url).unwrap_or_else(|| "playlist".to_string());

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let payload = json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": "2.20260708.00.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "browseId": if playlist_id.starts_with("VL") { playlist_id.clone() } else { format!("VL{}", playlist_id) }
        });

        let response = self
            .http
            .post("https://www.youtube.com/youtubei/v1/browse")
            .headers(headers.clone())
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "YouTube browse API request failed with HTTP {}",
                response.status()
            )));
        }

        let val: Value = response.json().await?;
        let (title, uploader, mut items, mut next_token) = self.parse_tab_json(&val);

        // Continuation loop for infinite playlist pagination
        let mut page_count = 0;
        while let Some(token) = next_token.take() {
            if page_count >= 50 {
                break; // Cap pagination to prevent runaway loops
            }
            page_count += 1;

            let cont_payload = json!({
                "context": {
                    "client": {
                        "clientName": "WEB",
                        "clientVersion": "2.20260708.00.00",
                        "hl": "en",
                        "gl": "US"
                    }
                },
                "continuation": token
            });

            if let Ok(cont_resp) = self
                .http
                .post("https://www.youtube.com/youtubei/v1/browse")
                .headers(headers.clone())
                .json(&cont_payload)
                .send()
                .await
                && cont_resp.status().is_success()
                && let Ok(cont_val) = cont_resp.json::<Value>().await
            {
                let (_, _, page_items, new_token) = self.parse_tab_json(&cont_val);
                if page_items.is_empty() {
                    break;
                }
                items.extend(page_items);
                next_token = new_token;
            } else {
                break;
            }
        }

        Ok(PlaylistMetadata {
            id: playlist_id,
            title: title.unwrap_or_else(|| "YouTube Playlist".to_string()),
            description: None,
            uploader,
            items,
            webpage_url: url.to_string(),
        })
    }

    fn parse_tab_json(
        &self,
        val: &Value,
    ) -> (
        Option<String>,
        Option<String>,
        Vec<PlaylistItem>,
        Option<String>,
    ) {
        let title = val
            .pointer("/header/playlistHeaderRenderer/title/simpleText")
            .or_else(|| val.pointer("/header/playlistHeaderRenderer/title/runs/0/text"))
            .or_else(|| val.pointer("/metadata/playlistMetadataRenderer/title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let uploader = val
            .pointer("/header/playlistHeaderRenderer/ownerText/runs/0/text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut items = Vec::new();
        let mut next_token = None;
        self.collect_video_renderers(val, &mut items, &mut next_token);

        (title, uploader, items, next_token)
    }

    fn collect_video_renderers(
        &self,
        val: &Value,
        items: &mut Vec<PlaylistItem>,
        next_token: &mut Option<String>,
    ) {
        if let Some(arr) = val.as_array() {
            for item in arr {
                self.collect_video_renderers(item, items, next_token);
            }
        } else if let Some(obj) = val.as_object() {
            if let Some(renderer) = obj
                .get("playlistVideoRenderer")
                .or_else(|| obj.get("compactVideoRenderer"))
                .or_else(|| obj.get("gridVideoRenderer"))
            {
                if let Some(v_id) = renderer.get("videoId").and_then(|v| v.as_str()) {
                    let title = renderer
                        .pointer("/title/runs/0/text")
                        .or_else(|| renderer.pointer("/title/simpleText"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled")
                        .to_string();
                    let uploader = renderer
                        .pointer("/shortBylineText/runs/0/text")
                        .or_else(|| renderer.pointer("/longBylineText/runs/0/text"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let duration = renderer
                        .get("lengthSeconds")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok());

                    let thumb = renderer
                        .pointer("/thumbnail/thumbnails/0/url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    items.push(PlaylistItem {
                        id: VideoId::new(v_id),
                        title: Some(title),
                        duration,
                        uploader,
                        thumbnail: thumb,
                    });
                }
            } else if let Some(cont_item) = obj.get("continuationItemRenderer") {
                if let Some(token) = cont_item
                    .pointer("/continuationEndpoint/continuationCommand/token")
                    .and_then(|v| v.as_str())
                {
                    *next_token = Some(token.to_string());
                }
            } else {
                for (_, v) in obj {
                    self.collect_video_renderers(v, items, next_token);
                }
            }
        }
    }
}
