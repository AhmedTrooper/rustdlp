use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::extractor::youtube::innertube::InnertubeClientKind;
use crate::models::innertube::InnertubePlayerResponse;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

pub struct InnertubeClient {
    http: reqwest::Client,
    client_kind: InnertubeClientKind,
}

impl InnertubeClient {
    pub fn new(http: reqwest::Client, client_kind: InnertubeClientKind) -> Self {
        Self { http, client_kind }
    }

    pub async fn get_player_response(
        &self,
        video_id: &VideoId,
        visitor_data: Option<&str>,
    ) -> Result<InnertubePlayerResponse> {
        let payload = self
            .client_kind
            .build_payload(video_id.as_str(), visitor_data);

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(self.client_kind.user_agent())
                .unwrap_or_else(|_| HeaderValue::from_static("Mozilla/5.0")),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-YouTube-Client-Name",
            HeaderValue::from_static(self.client_kind.client_id_num()),
        );
        headers.insert(
            "X-YouTube-Client-Version",
            HeaderValue::from_static(self.client_kind.client_version()),
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
                self.client_kind.client_name(),
                response.status()
            )));
        }

        let raw: Value = response.json().await?;
        let parsed: InnertubePlayerResponse = serde_json::from_value(raw)?;
        Ok(parsed)
    }
}
