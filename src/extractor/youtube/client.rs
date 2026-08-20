use crate::core::error::{DlpError, Result};
use crate::core::types::VideoId;
use crate::models::innertube::InnertubePlayerResponse;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, ORIGIN, USER_AGENT};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InnertubeClientType {
    VisionOs,
    AndroidVr,
    Android,
    Ios,
    Web,
}

impl InnertubeClientType {
    pub fn name(&self) -> &'static str {
        match self {
            InnertubeClientType::VisionOs => "visionos",
            InnertubeClientType::AndroidVr => "android_vr",
            InnertubeClientType::Android => "android",
            InnertubeClientType::Ios => "ios",
            InnertubeClientType::Web => "web",
        }
    }

    pub fn client_name(&self) -> &'static str {
        match self {
            InnertubeClientType::VisionOs => "VISIONOS",
            InnertubeClientType::AndroidVr => "ANDROID_VR",
            InnertubeClientType::Android => "ANDROID",
            InnertubeClientType::Ios => "IOS",
            InnertubeClientType::Web => "WEB",
        }
    }

    pub fn client_version(&self) -> &'static str {
        match self {
            InnertubeClientType::VisionOs => "1.02",
            InnertubeClientType::AndroidVr => "1.65.10",
            InnertubeClientType::Android => "21.26.364",
            InnertubeClientType::Ios => "21.26.4",
            InnertubeClientType::Web => "2.20260708.00.00",
        }
    }

    pub fn client_id_header(&self) -> &'static str {
        match self {
            InnertubeClientType::VisionOs => "101",
            InnertubeClientType::AndroidVr => "28",
            InnertubeClientType::Android => "3",
            InnertubeClientType::Ios => "5",
            InnertubeClientType::Web => "1",
        }
    }

    pub fn user_agent(&self) -> &'static str {
        match self {
            InnertubeClientType::VisionOs => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15"
            }
            InnertubeClientType::AndroidVr => {
                "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip"
            }
            InnertubeClientType::Android => {
                "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip"
            }
            InnertubeClientType::Ios => {
                "com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)"
            }
            InnertubeClientType::Web => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
            }
        }
    }

    pub fn build_payload(&self, video_id: &VideoId) -> serde_json::Value {
        let client_context = match self {
            InnertubeClientType::VisionOs => json!({
                "clientName": self.client_name(),
                "clientVersion": self.client_version(),
                "deviceMake": "Apple",
                "deviceModel": "RealityDevice17,1",
                "userAgent": self.user_agent(),
                "osName": "visionOS",
                "osVersion": "26.5.23O471",
                "hl": "en",
                "timeZone": "UTC",
                "utcOffsetMinutes": 0
            }),
            InnertubeClientType::AndroidVr => json!({
                "clientName": self.client_name(),
                "clientVersion": self.client_version(),
                "deviceMake": "Oculus",
                "deviceModel": "Quest 3",
                "androidSdkVersion": 32,
                "userAgent": self.user_agent(),
                "osName": "Android",
                "osVersion": "12L",
                "hl": "en",
                "timeZone": "UTC",
                "utcOffsetMinutes": 0
            }),
            InnertubeClientType::Android => json!({
                "clientName": self.client_name(),
                "clientVersion": self.client_version(),
                "androidSdkVersion": 30,
                "userAgent": self.user_agent(),
                "osName": "Android",
                "osVersion": "11",
                "hl": "en",
                "timeZone": "UTC",
                "utcOffsetMinutes": 0
            }),
            InnertubeClientType::Ios => json!({
                "clientName": self.client_name(),
                "clientVersion": self.client_version(),
                "deviceMake": "Apple",
                "deviceModel": "iPhone16,2",
                "userAgent": self.user_agent(),
                "osName": "iPhone",
                "osVersion": "18.3.2.22D82",
                "hl": "en",
                "timeZone": "UTC",
                "utcOffsetMinutes": 0
            }),
            InnertubeClientType::Web => json!({
                "clientName": self.client_name(),
                "clientVersion": self.client_version(),
                "hl": "en",
                "timeZone": "UTC",
                "utcOffsetMinutes": 0
            }),
        };

        json!({
            "context": {
                "client": client_context
            },
            "videoId": video_id.as_str(),
            "playbackContext": {
                "contentPlaybackContext": {
                    "html5Preference": "HTML5_PREF_WANTS"
                }
            }
        })
    }

    pub fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static(self.user_agent()));
        headers.insert(
            "X-YouTube-Client-Name",
            HeaderValue::from_static(self.client_id_header()),
        );
        headers.insert(
            "X-YouTube-Client-Version",
            HeaderValue::from_static(self.client_version()),
        );
        headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));
        headers
    }
}

pub struct InnertubeClient {
    http: reqwest::Client,
}

impl InnertubeClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    pub fn with_http(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn fetch_player(
        &self,
        video_id: &VideoId,
        client_type: InnertubeClientType,
    ) -> Result<InnertubePlayerResponse> {
        let url = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
        let payload = client_type.build_payload(video_id);
        let headers = client_type.build_headers();

        let response = self
            .http
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Innertube API returned HTTP status {}",
                status
            )));
        }

        let player_res: InnertubePlayerResponse = response.json().await?;
        Ok(player_res)
    }
}
