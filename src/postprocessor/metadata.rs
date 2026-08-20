use crate::core::error::Result;
use crate::models::video::VideoMetadata;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct MetadataWriter;

impl MetadataWriter {
    pub async fn write_info_json(metadata: &VideoMetadata, path: &Path) -> Result<()> {
        let json_str = serde_json::to_string_pretty(metadata)?;
        let mut file = File::create(path).await?;
        file.write_all(json_str.as_bytes()).await?;
        file.flush().await?;
        println!("[info] Writing video metadata to: {}", path.display());
        Ok(())
    }

    pub async fn download_thumbnail(
        http: &reqwest::Client,
        metadata: &VideoMetadata,
        path: &Path,
    ) -> Result<()> {
        if let Some(thumb_url) = metadata.best_thumbnail() {
            println!("[info] Downloading thumbnail to: {}", path.display());
            let resp = http.get(thumb_url).send().await?;
            let bytes = resp.bytes().await?;
            let mut file = File::create(path).await?;
            file.write_all(&bytes).await?;
            file.flush().await?;
        }
        Ok(())
    }
}
