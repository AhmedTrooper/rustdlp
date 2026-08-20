use crate::core::error::Result;
use crate::models::subtitle::{SubtitleConverter, SubtitleTrack};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct SubtitleDownloader {
    http: reqwest::Client,
}

impl SubtitleDownloader {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn download_subtitle(
        &self,
        track: &SubtitleTrack,
        format: &str,
        dest_path: &Path,
    ) -> Result<()> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
            ),
        );

        let target_url = if format == "vtt" {
            track.vtt_url()
        } else {
            track.srv3_url()
        };

        let resp = self.http.get(&target_url).headers(headers).send().await?;
        let content = resp.text().await?;

        let output_content = if format == "srt" {
            SubtitleConverter::timedtext_to_srt(&content)
        } else {
            SubtitleConverter::timedtext_to_vtt(&content)
        };

        let mut file = File::create(dest_path).await?;
        file.write_all(output_content.as_bytes()).await?;
        file.flush().await?;

        println!(
            "[info] Writing subtitle ({}) to: {}",
            track.language_code,
            dest_path.display()
        );

        Ok(())
    }
}
