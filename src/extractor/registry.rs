use crate::core::error::{DlpError, Result};
use crate::extractor::dailymotion::DailymotionExtractor;
use crate::extractor::facebook::FacebookExtractor;
use crate::extractor::generic::GenericExtractor;
use crate::extractor::instagram::InstagramExtractor;
use crate::extractor::linkedin::LinkedInExtractor;
use crate::extractor::reddit::RedditExtractor;
use crate::extractor::soundcloud::SoundCloudExtractor;
use crate::extractor::tiktok::TikTokExtractor;
use crate::extractor::traits::Extractor;
use crate::extractor::twitter::TwitterExtractor;
use crate::extractor::vimeo::VimeoExtractor;
use crate::extractor::youtube::YoutubeExtractor;
use crate::models::video::VideoMetadata;
use std::sync::Arc;

pub struct ExtractorRegistry {
    extractors: Vec<Arc<dyn Extractor>>,
    generic_extractor: Arc<dyn Extractor>,
}

impl ExtractorRegistry {
    pub fn new(http: reqwest::Client) -> Self {
        let extractors: Vec<Arc<dyn Extractor>> = vec![
            Arc::new(YoutubeExtractor::with_http_client(http.clone())),
            Arc::new(FacebookExtractor::with_http_client(http.clone())),
            Arc::new(TwitterExtractor::with_http_client(http.clone())),
            Arc::new(DailymotionExtractor::with_http_client(http.clone())),
            Arc::new(LinkedInExtractor::with_http_client(http.clone())),
            Arc::new(TikTokExtractor::with_http_client(http.clone())),
            Arc::new(InstagramExtractor::with_http_client(http.clone())),
            Arc::new(RedditExtractor::with_http_client(http.clone())),
            Arc::new(VimeoExtractor::with_http_client(http.clone())),
            Arc::new(SoundCloudExtractor::with_http_client(http.clone())),
        ];

        let generic_extractor: Arc<dyn Extractor> =
            Arc::new(GenericExtractor::with_http_client(http));

        Self {
            extractors,
            generic_extractor,
        }
    }

    pub fn register(&mut self, extractor: Arc<dyn Extractor>) {
        self.extractors.push(extractor);
    }

    pub fn find_extractor(&self, url: &str) -> Option<Arc<dyn Extractor>> {
        self.extractors
            .iter()
            .find(|e| e.can_extract(url))
            .cloned()
            .or_else(|| {
                if self.generic_extractor.can_extract(url) {
                    Some(self.generic_extractor.clone())
                } else {
                    None
                }
            })
    }

    pub async fn extract(&self, url: &str) -> Result<VideoMetadata> {
        if let Some(extractor) = self.find_extractor(url) {
            extractor.extract(url).await
        } else {
            Err(DlpError::UnsupportedUrl(format!(
                "No suitable extractor found for URL: {}",
                url
            )))
        }
    }
}
