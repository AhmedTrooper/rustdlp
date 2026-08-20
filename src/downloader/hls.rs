use crate::core::error::{DlpError, Result};
use crate::downloader::progress::DownloadProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

pub struct HlsDownloader {
    http: reqwest::Client,
}

impl HlsDownloader {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn download_m3u8(
        &self,
        manifest_url: &str,
        output_path: &Path,
        user_agent: Option<&str>,
    ) -> Result<()> {
        let ua = user_agent.unwrap_or(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(ua).unwrap_or(HeaderValue::from_static("Mozilla/5.0")),
        );

        let manifest_text = self
            .http
            .get(manifest_url)
            .headers(headers.clone())
            .send()
            .await?
            .text()
            .await?;

        let base_url = Url::parse(manifest_url)
            .map_err(|e| DlpError::DownloadError(format!("Invalid HLS manifest URL: {}", e)))?;

        // Check if master playlist containing sub-playlists
        let mut target_playlist_url = manifest_url.to_string();
        for line in manifest_text.lines() {
            let line = line.trim();
            if !line.starts_with('#') && !line.is_empty() {
                if let Ok(resolved) = base_url.join(line) {
                    target_playlist_url = resolved.to_string();
                    break;
                }
            }
        }

        // Fetch media playlist if master was given
        let media_playlist_text = if target_playlist_url != manifest_url {
            self.http
                .get(&target_playlist_url)
                .headers(headers.clone())
                .send()
                .await?
                .text()
                .await?
        } else {
            manifest_text
        };

        let media_base_url = Url::parse(&target_playlist_url)
            .map_err(|e| DlpError::DownloadError(format!("Invalid HLS media URL: {}", e)))?;

        let mut segment_urls = Vec::new();
        for line in media_playlist_text.lines() {
            let line = line.trim();
            if !line.starts_with('#') && !line.is_empty() {
                if let Ok(resolved) = media_base_url.join(line) {
                    segment_urls.push(resolved.to_string());
                }
            }
        }

        if segment_urls.is_empty() {
            return Err(DlpError::DownloadError("No segments found in HLS playlist".into()));
        }

        let total_segments = segment_urls.len() as u64;
        let progress_bar = DownloadProgressBar::new(Some(total_segments), "[hls]");

        let temp_ts_path = Self::get_temp_ts_path(output_path);
        let mut temp_file = File::create(&temp_ts_path).await?;

        for seg_url in segment_urls {
            let resp = self
                .http
                .get(&seg_url)
                .headers(headers.clone())
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(DlpError::DownloadError(format!(
                    "HLS segment download failed: {}",
                    resp.status()
                )));
            }

            let bytes = resp.bytes().await?;
            temp_file.write_all(&bytes).await?;
            progress_bar.inc(1);
        }

        temp_file.flush().await?;
        drop(temp_file);

        progress_bar.finish_and_clear();

        // Atomically rename or remux into output destination
        tokio::fs::rename(&temp_ts_path, output_path).await?;
        Ok(())
    }

    fn get_temp_ts_path(path: &Path) -> PathBuf {
        let mut p = path.to_path_buf();
        let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        p.set_file_name(format!("{}.ts.part", fname));
        p
    }
}
