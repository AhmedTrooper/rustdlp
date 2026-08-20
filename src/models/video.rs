use crate::core::types::VideoId;
use crate::models::format::StreamFormat;
use crate::models::subtitle::SubtitleTrack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub id: VideoId,
    pub title: String,
    pub uploader: String,
    pub channel_id: Option<String>,
    pub duration: Option<u64>,
    pub view_count: Option<u64>,
    pub description: Option<String>,
    pub upload_date: Option<String>,
    pub thumbnails: Vec<String>,
    pub formats: Vec<StreamFormat>,
    pub subtitles: Vec<SubtitleTrack>,
    pub webpage_url: String,
    pub is_live: bool,
}

impl VideoMetadata {
    pub fn best_thumbnail(&self) -> Option<&str> {
        self.thumbnails.last().map(|s| s.as_str())
    }

    pub fn video_formats(&self) -> Vec<&StreamFormat> {
        self.formats.iter().filter(|f| f.is_video()).collect()
    }

    pub fn audio_formats(&self) -> Vec<&StreamFormat> {
        self.formats.iter().filter(|f| f.is_audio()).collect()
    }

    pub fn find_format_by_id(&self, id: &str) -> Option<&StreamFormat> {
        self.formats.iter().find(|f| f.format_id.as_str() == id || f.itag.to_string() == id)
    }

    pub fn find_subtitle(&self, lang: &str) -> Option<&SubtitleTrack> {
        self.subtitles.iter().find(|s| s.language_code == lang || s.language_code.starts_with(lang))
    }
}
