use crate::core::error::Result;
use crate::downloader::http::HttpDownloader;
use crate::models::format::StreamFormat;
use crate::postprocessor::ffmpeg::FFmpeg;
use std::path::Path;

pub struct AdaptiveDownloader {
    http: HttpDownloader,
}

impl Default for AdaptiveDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveDownloader {
    pub fn new() -> Self {
        Self {
            http: HttpDownloader::new(),
        }
    }

    pub async fn download_and_merge(
        &self,
        video_format: &StreamFormat,
        audio_format: &StreamFormat,
        output_path: &Path,
        keep_fragments: bool,
    ) -> Result<()> {
        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = output_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        let video_temp = parent.join(format!(
            "{}.f{}.{}",
            stem, video_format.itag, video_format.ext
        ));
        let audio_temp = parent.join(format!(
            "{}.f{}.{}",
            stem, audio_format.itag, audio_format.ext
        ));

        println!(
            "[download] Downloading 2 streams (video itag {}, audio itag {})",
            video_format.itag, audio_format.itag
        );

        let v_prefix = format!("[download] itag {:<3}", video_format.itag);
        let a_prefix = format!("[download] itag {:<3}", audio_format.itag);

        // Download video and audio streams concurrently
        let v_download = self.http.download_to_file(
            &video_format.url,
            &video_temp,
            video_format.effective_filesize(),
            &v_prefix,
            Some(video_format.user_agent()),
        );

        let a_download = self.http.download_to_file(
            &audio_format.url,
            &audio_temp,
            audio_format.effective_filesize(),
            &a_prefix,
            Some(audio_format.user_agent()),
        );

        tokio::try_join!(v_download, a_download)?;

        // Merge video and audio with FFmpeg
        FFmpeg::merge_streams(&video_temp, &audio_temp, output_path).await?;

        // Cleanup temporary fragments
        if !keep_fragments {
            let _ = tokio::fs::remove_file(&video_temp).await;
            let _ = tokio::fs::remove_file(&audio_temp).await;
        }

        Ok(())
    }
}
