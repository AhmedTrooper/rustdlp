use crate::core::error::Result;
use crate::core::types::Protocol;
use crate::downloader::hls::HlsDownloader;
use crate::downloader::http::HttpDownloader;
use crate::downloader::traits::StreamDownloader;
use crate::models::format::StreamFormat;
use async_trait::async_trait;
use std::path::Path;

pub struct SingleStreamDownloader {
    http: HttpDownloader,
}

impl Default for SingleStreamDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl SingleStreamDownloader {
    pub fn new() -> Self {
        Self {
            http: HttpDownloader::new(),
        }
    }
}

#[async_trait]
impl StreamDownloader for SingleStreamDownloader {
    async fn download(&self, format: &StreamFormat, output_path: &Path) -> Result<()> {
        let prefix = format!("[download] itag {:<3}", format.itag);

        if format.protocol == Protocol::Hls || format.url.contains(".m3u8") {
            let hls = HlsDownloader::new(reqwest::Client::new());
            hls.download_m3u8(&format.url, output_path, Some(format.user_agent()))
                .await
        } else {
            self.http
                .download_to_file(
                    &format.url,
                    output_path,
                    format.effective_filesize(),
                    &prefix,
                    Some(format.user_agent()),
                )
                .await
        }
    }
}
