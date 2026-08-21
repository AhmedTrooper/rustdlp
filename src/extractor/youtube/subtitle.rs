use crate::core::error::{DlpError, Result};
use crate::models::subtitle::{SubtitleConverter, SubtitleTrack};
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use std::path::Path;

pub struct SubtitleDownloader {
    http: reqwest::Client,
}

impl Default for SubtitleDownloader {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl SubtitleDownloader {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub fn with_http_client(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn download_subtitle(
        &self,
        track: &SubtitleTrack,
        target_format: &str,
        output_path: &Path,
    ) -> Result<()> {
        let fetch_url = match target_format {
            "vtt" => track.vtt_url(),
            "srv3" | "xml" => track.srv3_url(),
            _ => track.vtt_url(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

        let response = self.http.get(&fetch_url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(DlpError::ExtractionError(format!(
                "Failed to download subtitle from {} with HTTP {}",
                fetch_url,
                response.status()
            )));
        }

        let raw_content = response.text().await?;

        let formatted = if target_format == "srt" {
            SubtitleConverter::timedtext_to_srt(&raw_content)
        } else if target_format == "vtt" {
            SubtitleConverter::timedtext_to_vtt(&raw_content)
        } else {
            raw_content
        };

        tokio::fs::write(output_path, formatted).await?;

        Ok(())
    }

    pub fn generate_translated_tracks(
        track: &SubtitleTrack,
        target_languages: &[(&str, &str)],
    ) -> Vec<SubtitleTrack> {
        let mut translated = Vec::new();
        for &(lang_code, lang_name) in target_languages {
            if lang_code == track.language_code {
                continue;
            }

            let translated_url = if track.base_url.contains("tlang=") {
                track.base_url.clone()
            } else {
                format!("{}&tlang={}", track.base_url, lang_code)
            };

            translated.push(SubtitleTrack {
                language_code: lang_code.to_string(),
                name: format!("{} (translated from {})", lang_name, track.name),
                base_url: translated_url,
                is_auto_generated: true,
            });
        }
        translated
    }
}
