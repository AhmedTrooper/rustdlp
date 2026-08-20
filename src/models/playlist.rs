use crate::core::types::VideoId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: VideoId,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<u64>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistMetadata {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub items: Vec<PlaylistItem>,
    pub webpage_url: String,
}

impl PlaylistMetadata {
    pub fn video_count(&self) -> usize {
        self.items.len()
    }
}
