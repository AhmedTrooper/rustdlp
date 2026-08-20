use crate::core::error::Result;
use crate::models::format::StreamFormat;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait StreamDownloader: Send + Sync {
    async fn download(&self, format: &StreamFormat, output_path: &Path) -> Result<()>;
}
