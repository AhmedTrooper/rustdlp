use crate::core::types::{FormatId, MediaType, Protocol, Resolution};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamFormat {
    pub itag: u32,
    pub format_id: FormatId,
    pub url: String,
    pub ext: String,
    pub resolution: Resolution,
    pub fps: Option<u32>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub bitrate: Option<u64>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub media_type: MediaType,
    pub protocol: Protocol,
    pub quality_label: Option<String>,
    pub audio_channels: Option<u32>,
    pub audio_sample_rate: Option<u32>,
    pub source_client: String,
}

impl StreamFormat {
    pub fn is_video(&self) -> bool {
        self.media_type == MediaType::VideoOnly || self.media_type == MediaType::Combined
    }

    pub fn is_audio(&self) -> bool {
        self.media_type == MediaType::AudioOnly || self.media_type == MediaType::Combined
    }

    pub fn is_video_only(&self) -> bool {
        self.media_type == MediaType::VideoOnly
    }

    pub fn is_audio_only(&self) -> bool {
        self.media_type == MediaType::AudioOnly
    }

    pub fn is_combined(&self) -> bool {
        self.media_type == MediaType::Combined
    }

    pub fn effective_filesize(&self) -> Option<u64> {
        self.filesize.or(self.filesize_approx)
    }

    pub fn height(&self) -> u32 {
        self.resolution.height.unwrap_or(0)
    }

    pub fn width(&self) -> u32 {
        self.resolution.width.unwrap_or(0)
    }

    pub fn effective_fps(&self) -> u32 {
        self.fps.unwrap_or(0)
    }

    pub fn effective_bitrate(&self) -> u64 {
        self.bitrate.unwrap_or(0)
    }

    pub fn user_agent(&self) -> &'static str {
        match self.source_client.as_str() {
            "visionos" => {
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15"
            }
            "android_vr" => {
                "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip"
            }
            "android" => {
                "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip"
            }
            "ios" => {
                "com.google.ios.youtube/21.26.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)"
            }
            _ => {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
            }
        }
    }
}
