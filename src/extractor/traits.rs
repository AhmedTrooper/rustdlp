use crate::core::error::Result;
use crate::models::video::VideoMetadata;
use async_trait::async_trait;

#[async_trait]
pub trait Extractor: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_extract(&self, url: &str) -> bool;
    async fn extract(&self, url: &str) -> Result<VideoMetadata>;
}
