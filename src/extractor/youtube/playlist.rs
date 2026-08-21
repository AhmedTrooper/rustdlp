use crate::core::error::Result;
use crate::extractor::youtube::tab::YoutubeTabExtractor;
use crate::models::playlist::PlaylistMetadata;

pub struct YoutubePlaylistExtractor {
    tab_extractor: YoutubeTabExtractor,
}

impl YoutubePlaylistExtractor {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            tab_extractor: YoutubeTabExtractor::with_http_client(http),
        }
    }

    pub fn with_http_client(http: reqwest::Client) -> Self {
        Self::new(http)
    }

    pub async fn extract_playlist(&self, playlist_id: &str) -> Result<PlaylistMetadata> {
        let url = format!("https://www.youtube.com/playlist?list={}", playlist_id);
        self.tab_extractor.extract_tab(&url).await
    }

    pub async fn extract(&self, url: &str) -> Result<PlaylistMetadata> {
        self.tab_extractor.extract_tab(url).await
    }
}
