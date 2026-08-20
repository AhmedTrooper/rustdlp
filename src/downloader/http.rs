use crate::core::error::{DlpError, Result};
use crate::downloader::progress::DownloadProgressBar;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE, USER_AGENT};
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

const CHUNK_SIZE: u64 = 5 * 1024 * 1024; // 5 MiB chunks for maximum un-throttled throughput

pub struct HttpDownloader {
    http: reqwest::Client,
}

impl HttpDownloader {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    pub async fn download_to_file(
        &self,
        url: &str,
        dest_path: &Path,
        expected_size: Option<u64>,
        prefix: &str,
        user_agent: Option<&str>,
    ) -> Result<()> {
        let part_path = Self::get_part_path(dest_path);

        // Check if destination already exists and is complete
        if dest_path.exists() {
            if let Ok(meta) = std::fs::metadata(dest_path) {
                if let Some(expected) = expected_size {
                    if meta.len() == expected {
                        println!("[download] {} already fully downloaded", dest_path.display());
                        return Ok(());
                    }
                }
            }
        }

        let mut total_size = expected_size;
        let ua = user_agent.unwrap_or(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );

        // If total size unknown, probe with a small range request
        if total_size.is_none() {
            let mut probe_headers = HeaderMap::new();
            probe_headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap_or(HeaderValue::from_static("Mozilla/5.0")));
            probe_headers.insert(RANGE, HeaderValue::from_static("bytes=0-1"));

            if let Ok(probe_resp) = self.http.get(url).headers(probe_headers).send().await {
                if let Some(content_range) = probe_resp.headers().get("content-range").and_then(|v| v.to_str().ok()) {
                    // Content-Range: bytes 0-1/1234567
                    if let Some((_, total_str)) = content_range.split_once('/') {
                        total_size = total_str.parse::<u64>().ok();
                    }
                }
            }
        }

        // Determine resume offset from existing .part file
        let mut downloaded_bytes = 0u64;
        let mut open_options = OpenOptions::new();
        open_options.create(true).write(true);

        if part_path.exists() {
            if let Ok(meta) = std::fs::metadata(&part_path) {
                downloaded_bytes = meta.len();
                if Some(downloaded_bytes) == total_size && total_size.is_some() {
                    tokio::fs::rename(&part_path, dest_path).await?;
                    return Ok(());
                }
                open_options.append(true);
            } else {
                open_options.truncate(true);
            }
        } else {
            open_options.truncate(true);
        }

        let progress_bar = DownloadProgressBar::new(total_size, prefix);
        if downloaded_bytes > 0 {
            progress_bar.inc(downloaded_bytes);
        }

        let mut file = open_options.open(&part_path).await?;

        // Download in chunks or fallback to streaming
        if let Some(total) = total_size {
            let mut curr_offset = downloaded_bytes;
            while curr_offset < total {
                let chunk_end = (curr_offset + CHUNK_SIZE - 1).min(total - 1);
                let range_header = format!("bytes={}-{}", curr_offset, chunk_end);

                let mut headers = HeaderMap::new();
                headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap_or(HeaderValue::from_static("Mozilla/5.0")));
                headers.insert(RANGE, HeaderValue::from_str(&range_header).map_err(|e| DlpError::DownloadError(e.to_string()))?);

                let response = self.http.get(url).headers(headers).send().await?;
                let status = response.status();

                if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
                    return Err(DlpError::DownloadError(format!("Chunk download failed with status {}", status)));
                }

                let mut stream = response.bytes_stream();
                while let Some(chunk_res) = stream.next().await {
                    let chunk = chunk_res.map_err(DlpError::Network)?;
                    file.write_all(&chunk).await?;
                    let chunk_len = chunk.len() as u64;
                    curr_offset += chunk_len;
                    progress_bar.inc(chunk_len);
                }
            }
        } else {
            // Streaming mode if total size cannot be determined
            let mut headers = HeaderMap::new();
            headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap_or(HeaderValue::from_static("Mozilla/5.0")));
            if downloaded_bytes > 0 {
                headers.insert(RANGE, HeaderValue::from_str(&format!("bytes={}-", downloaded_bytes)).map_err(|e| DlpError::DownloadError(e.to_string()))?);
            }

            let response = self.http.get(url).headers(headers).send().await?;
            let mut stream = response.bytes_stream();

            while let Some(chunk_res) = stream.next().await {
                let chunk = chunk_res.map_err(DlpError::Network)?;
                file.write_all(&chunk).await?;
                progress_bar.inc(chunk.len() as u64);
            }
        }

        file.flush().await?;
        drop(file);

        progress_bar.finish_and_clear();

        // Atomically rename .part file to destination
        tokio::fs::rename(&part_path, dest_path).await?;
        Ok(())
    }

    fn get_part_path(path: &Path) -> PathBuf {
        let mut part = path.to_path_buf();
        let filename = part.file_name().unwrap_or_default().to_string_lossy().to_string();
        part.set_file_name(format!("{}.part", filename));
        part
    }
}
