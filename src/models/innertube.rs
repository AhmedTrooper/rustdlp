use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnertubePlayerResponse {
    #[serde(rename = "playabilityStatus")]
    pub playability_status: Option<PlayabilityStatus>,

    #[serde(rename = "videoDetails")]
    pub video_details: Option<VideoDetails>,

    #[serde(rename = "streamingData")]
    pub streaming_data: Option<StreamingData>,

    #[serde(rename = "microformat")]
    pub microformat: Option<Microformat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayabilityStatus {
    pub status: Option<String>,
    pub reason: Option<String>,
    #[serde(rename = "playableInEmbed")]
    pub playable_in_embed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDetails {
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "lengthSeconds")]
    pub length_seconds: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    pub author: Option<String>,
    #[serde(rename = "viewCount")]
    pub view_count: Option<String>,
    #[serde(rename = "shortDescription")]
    pub short_description: Option<String>,
    pub thumbnail: Option<ThumbnailContainer>,
    #[serde(rename = "isLiveContent")]
    pub is_live_content: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailContainer {
    pub thumbnails: Option<Vec<ThumbnailItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailItem {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingData {
    #[serde(rename = "expiresInSeconds")]
    pub expires_in_seconds: Option<String>,
    pub formats: Option<Vec<RawFormatStream>>,
    #[serde(rename = "adaptiveFormats")]
    pub adaptive_formats: Option<Vec<RawFormatStream>>,
    #[serde(rename = "hlsManifestUrl")]
    pub hls_manifest_url: Option<String>,
    #[serde(rename = "dashManifestUrl")]
    pub dash_manifest_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFormatStream {
    pub itag: u32,
    pub url: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub bitrate: Option<u64>,
    #[serde(rename = "averageBitrate")]
    pub average_bitrate: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    #[serde(rename = "contentLength")]
    pub content_length: Option<String>,
    pub quality: Option<String>,
    #[serde(rename = "qualityLabel")]
    pub quality_label: Option<String>,
    pub fps: Option<u32>,
    #[serde(rename = "audioQuality")]
    pub audio_quality: Option<String>,
    #[serde(rename = "approxDurationMs")]
    pub approx_duration_ms: Option<String>,
    #[serde(rename = "audioSampleRate")]
    pub audio_sample_rate: Option<String>,
    #[serde(rename = "audioChannels")]
    pub audio_channels: Option<u32>,
    #[serde(rename = "signatureCipher")]
    pub signature_cipher: Option<String>,
    pub cipher: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Microformat {
    #[serde(rename = "playerMicroformatRenderer")]
    pub player_microformat_renderer: Option<PlayerMicroformatRenderer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerMicroformatRenderer {
    #[serde(rename = "uploadDate")]
    pub upload_date: Option<String>,
    #[serde(rename = "publishDate")]
    pub publish_date: Option<String>,
    pub category: Option<String>,
}
