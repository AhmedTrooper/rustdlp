use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::extractor::traits::Extractor;
use crate::extractor::youtube::dash_mpd::YoutubeDashMpdParser;
use crate::extractor::youtube::innertube::InnertubeClientKind;
use crate::extractor::youtube::jsc::JsChallengeSolver;
use crate::extractor::youtube::metadata::YoutubeMetadataParser;
use crate::extractor::youtube::parser::YoutubeParser;
use crate::extractor::youtube::url::{extract_youtube_video_id, is_youtube_url};
use crate::models::format::StreamFormat;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;
use std::collections::HashSet;

pub struct YoutubeExtractor {
    http: reqwest::Client,
}

impl Default for YoutubeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl YoutubeExtractor {
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

    pub async fn extract_by_id(&self, video_id: &VideoId) -> Result<VideoMetadata> {
        self.extract(video_id.as_str()).await
    }

    async fn fetch_innertube(
        &self,
        client: InnertubeClientKind,
        video_id: &str,
        visitor_data: Option<&str>,
    ) -> Result<Value> {
        let payload = client.build_payload(video_id, visitor_data);

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(client.user_agent())
                .unwrap_or_else(|_| HeaderValue::from_static("Mozilla/5.0")),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-YouTube-Client-Name",
            HeaderValue::from_static(client.client_id_num()),
        );
        headers.insert(
            "X-YouTube-Client-Version",
            HeaderValue::from_static(client.client_version()),
        );

        if let Some(v_data) = visitor_data
            && let Ok(hv) = HeaderValue::from_str(v_data)
        {
            headers.insert("X-Goog-Visitor-Id", hv);
        }

        let response = self
            .http
            .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Innertube {} request failed with HTTP {}",
                client.client_name(),
                response.status()
            )));
        }

        let val: Value = response.json().await?;
        Ok(val)
    }
}

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn can_extract(&self, url: &str) -> bool {
        is_youtube_url(url)
    }

    async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        let video_id = extract_youtube_video_id(url)?;
        let mut formats: Vec<StreamFormat> = Vec::new();
        let mut seen_itags = HashSet::new();

        let mut title: Option<String> = None;
        let mut uploader: Option<String> = None;
        let mut channel_id: Option<String> = None;
        let mut duration: Option<u64> = None;
        let mut view_count: Option<u64> = None;
        let mut description: Option<String> = None;
        let mut upload_date: Option<String> = None;
        let mut thumbnails = Vec::new();
        let mut rich_meta = None;
        let mut visitor_data_token: Option<String> = None;

        let solver = JsChallengeSolver::new();

        // 1. Query Innertube multi-clients (14 clients)
        for &client in InnertubeClientKind::all() {
            if let Ok(val) = self
                .fetch_innertube(client, video_id.as_str(), visitor_data_token.as_deref())
                .await
            {
                if visitor_data_token.is_none() {
                    visitor_data_token = val
                        .pointer("/responseContext/visitorData")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }

                // Check playability status
                let playability_status = val
                    .pointer("/playabilityStatus/status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("OK");

                if playability_status == "OK" {
                    if title.is_none() {
                        title = val
                            .pointer("/videoDetails/title")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                    if uploader.is_none() {
                        uploader = val
                            .pointer("/videoDetails/author")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                    if channel_id.is_none() {
                        channel_id = val
                            .pointer("/videoDetails/channelId")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                    if duration.is_none() {
                        duration = val
                            .pointer("/videoDetails/lengthSeconds")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<u64>().ok());
                    }
                    if view_count.is_none() {
                        view_count = val
                            .pointer("/videoDetails/viewCount")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<u64>().ok());
                    }
                    if description.is_none() {
                        description = val
                            .pointer("/videoDetails/shortDescription")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                    if upload_date.is_none() {
                        upload_date = val
                            .pointer("/microformat/playerMicroformatRenderer/publishDate")
                            .and_then(|v| v.as_str())
                            .map(|s| s.replace('-', ""));
                    }

                    if thumbnails.is_empty()
                        && let Some(thumbs) = val
                            .pointer("/videoDetails/thumbnail/thumbnails")
                            .and_then(|v| v.as_array())
                    {
                        for t in thumbs {
                            if let Some(u) = t.get("url").and_then(|v| v.as_str()) {
                                thumbnails.push(u.to_string());
                            }
                        }
                    }

                    if rich_meta.is_none() {
                        rich_meta = Some(YoutubeMetadataParser::parse(&val, None));
                    }

                    // Extract formats from streamingData
                    let client_formats = YoutubeParser::parse_player_response(&val, Some(&solver));
                    for mut f in client_formats {
                        if !seen_itags.contains(&f.itag) {
                            seen_itags.insert(f.itag);

                            // Apply n-param challenge bypass
                            if let Ok(mut parsed_url) = url::Url::parse(&f.url) {
                                let mut query_pairs: Vec<(String, String)> = parsed_url
                                    .query_pairs()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect();
                                for (k, v) in &mut query_pairs {
                                    if k == "n" {
                                        *v = solver.solve_n_param(v);
                                    }
                                }
                                parsed_url.query_pairs_mut().clear().extend_pairs(
                                    query_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
                                );
                                f.url = parsed_url.to_string();
                            }

                            formats.push(f);
                        }
                    }

                    // Parse DASH manifest URL if present
                    if let Some(dash_url) = val
                        .pointer("/streamingData/dashManifestUrl")
                        .and_then(|v| v.as_str())
                        && let Ok(dash_resp) = self.http.get(dash_url).send().await
                        && let Ok(dash_xml) = dash_resp.text().await
                    {
                        let dash_formats = YoutubeDashMpdParser::parse(&dash_xml, dash_url);
                        for df in dash_formats {
                            if !seen_itags.contains(&df.itag) {
                                seen_itags.insert(df.itag);
                                formats.push(df);
                            }
                        }
                    }

                    if !formats.is_empty() {
                        break;
                    }
                }
            }
        }

        if formats.is_empty() {
            return Err(DlpError::VideoUnavailable(format!(
                "No downloadable video formats found for YouTube ID: {}",
                video_id
            )));
        }

        let title_str = title.unwrap_or_else(|| format!("YouTube video #{}", video_id));
        let uploader_str = uploader.unwrap_or_else(|| "YouTube Creator".to_string());
        let subs = rich_meta
            .as_ref()
            .map(|m| m.subtitles.clone())
            .unwrap_or_default();
        let is_live_val = rich_meta.as_ref().map(|m| m.is_live).unwrap_or(false);

        Ok(VideoMetadata {
            id: video_id,
            title: title_str,
            uploader: uploader_str,
            channel_id,
            duration,
            view_count,
            description,
            upload_date,
            thumbnails,
            formats,
            subtitles: subs,
            webpage_url: url.to_string(),
            is_live: is_live_val,
        })
    }
}
