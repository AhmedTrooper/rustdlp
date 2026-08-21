use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InnertubeClientKind {
    VisionOs,
    Android,
    Ios,
    Web,
    WebSafari,
    WebEmbedded,
    WebMusic,
    WebCreator,
    AndroidVr,
    AndroidTestSuite,
    MWeb,
    Tv,
    TvEmbedded,
    TvDowngraded,
}

impl InnertubeClientKind {
    pub fn all() -> &'static [InnertubeClientKind] {
        &[
            Self::VisionOs,
            Self::Android,
            Self::Ios,
            Self::Web,
            Self::WebSafari,
            Self::WebEmbedded,
            Self::WebMusic,
            Self::WebCreator,
            Self::AndroidVr,
            Self::AndroidTestSuite,
            Self::MWeb,
            Self::Tv,
            Self::TvEmbedded,
            Self::TvDowngraded,
        ]
    }

    pub fn client_name(&self) -> &'static str {
        match self {
            Self::Web => "WEB",
            Self::WebSafari => "WEB",
            Self::WebEmbedded => "WEB_EMBEDDED_PLAYER",
            Self::WebMusic => "WEB_REMIX",
            Self::WebCreator => "WEB_CREATOR",
            Self::Android => "ANDROID",
            Self::AndroidVr => "ANDROID_VR",
            Self::AndroidTestSuite => "ANDROID_TESTSUITE",
            Self::Ios => "IOS",
            Self::VisionOs => "VISIONOS",
            Self::MWeb => "MWEB",
            Self::Tv => "TVHTML5",
            Self::TvEmbedded => "TVHTML5_SIMPLY_EMBEDDED_PLAYER",
            Self::TvDowngraded => "TV_DOWNGRADED",
        }
    }

    pub fn client_version(&self) -> &'static str {
        match self {
            Self::Web | Self::WebSafari | Self::WebEmbedded => "2.20260708.00.00",
            Self::WebMusic => "1.20260707.12.00",
            Self::WebCreator => "1.20260708.01.00",
            Self::Android => "21.26.364",
            Self::AndroidVr => "1.65.10",
            Self::AndroidTestSuite => "1.9.0",
            Self::Ios => "21.26.4",
            Self::VisionOs => "1.02",
            Self::MWeb => "2.20260708.05.00",
            Self::Tv => "7.20260707.08.00",
            Self::TvEmbedded => "2.0",
            Self::TvDowngraded => "6.20240101.00.00",
        }
    }

    pub fn client_id_num(&self) -> &'static str {
        match self {
            Self::Web | Self::WebSafari => "1",
            Self::Android => "3",
            Self::Ios => "5",
            Self::AndroidVr => "28",
            Self::WebEmbedded => "56",
            Self::WebMusic => "67",
            Self::WebCreator => "62",
            Self::VisionOs => "101",
            Self::MWeb => "65",
            Self::Tv => "31",
            Self::TvEmbedded => "85",
            Self::AndroidTestSuite => "89",
            Self::TvDowngraded => "31",
        }
    }

    pub fn user_agent(&self) -> &'static str {
        match self {
            Self::Web => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
            }
            Self::WebSafari => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15,gzip(gfe)"
            }
            Self::WebEmbedded => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
            }
            Self::WebMusic => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0"
            }
            Self::WebCreator => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
            }
            Self::Android => "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip",
            Self::AndroidVr => {
                "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip"
            }
            Self::AndroidTestSuite => {
                "google/coral/coral:10/QQ3A.200805.001/6578210:user/release-keys"
            }
            Self::Ios => {
                "com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)"
            }
            Self::VisionOs => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15"
            }
            Self::MWeb => {
                "Mozilla/5.0 (iPad; CPU OS 16_7_10 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1,gzip(gfe)"
            }
            Self::Tv => {
                "Mozilla/5.0 (ChromiumStylePlatform; Linux x86_64; GoogleTV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            }
            Self::TvEmbedded => {
                "Mozilla/5.0 (ChromiumStylePlatform; Linux x86_64; GoogleTV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            }
            Self::TvDowngraded => {
                "Mozilla/5.0 (ChromiumStylePlatform; Linux x86_64; GoogleTV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36"
            }
        }
    }

    pub fn build_payload(
        &self,
        video_id: &str,
        visitor_data: Option<&str>,
        sts: Option<u64>,
    ) -> Value {
        let mut client_obj = json!({
            "clientName": self.client_name(),
            "clientVersion": self.client_version(),
            "hl": "en",
            "gl": "US",
        });

        if let Some(v_data) = visitor_data {
            client_obj["visitorData"] = json!(v_data);
        }

        match self {
            Self::Android | Self::AndroidTestSuite => {
                client_obj["androidSdkVersion"] = json!(30);
                client_obj["osName"] = json!("Android");
                client_obj["osVersion"] = json!("11");
            }
            Self::AndroidVr => {
                client_obj["deviceMake"] = json!("Oculus");
                client_obj["deviceModel"] = json!("Quest 3");
                client_obj["androidSdkVersion"] = json!(32);
                client_obj["osName"] = json!("Android");
                client_obj["osVersion"] = json!("12L");
            }
            Self::Ios => {
                client_obj["deviceMake"] = json!("Apple");
                client_obj["deviceModel"] = json!("iPhone16,2");
                client_obj["osName"] = json!("iPhone");
                client_obj["osVersion"] = json!("18.3.2.22D82");
            }
            Self::VisionOs => {
                client_obj["deviceMake"] = json!("Apple");
                client_obj["deviceModel"] = json!("RealityDevice17,1");
                client_obj["osName"] = json!("visionOS");
                client_obj["osVersion"] = json!("26.5.23O471");
            }
            _ => {}
        }

        let sig_ts = sts.unwrap_or(19999);

        json!({
            "videoId": video_id,
            "context": {
                "client": client_obj,
                "user": {
                    "lockedSafetyMode": false
                },
                "request": {
                    "useSsl": true,
                    "internalExperimentFlags": []
                }
            },
            "playbackContext": {
                "contentPlaybackContext": {
                    "html5Preference": "HTML5_PREF_WANTS",
                    "signatureTimestamp": sig_ts
                }
            },
            "contentCheckOk": true,
            "racyCheckOk": true
        })
    }
}
