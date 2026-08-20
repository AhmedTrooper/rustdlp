use crate::core::error::{DlpError, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

pub struct FFmpeg;

impl FFmpeg {
    pub async fn is_available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Merges separate video and audio files into a single container
    pub async fn merge_streams(
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<()> {
        if !Self::is_available().await {
            return Err(DlpError::FFmpegError(
                "ffmpeg executable not found in PATH. Please install ffmpeg to merge video and audio streams.".into(),
            ));
        }

        println!(
            "[Merger] Merging formats into \"{}\"",
            output_path.display()
        );

        let output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(video_path)
            .arg("-i")
            .arg(audio_path)
            .arg("-c:v")
            .arg("copy")
            .arg("-c:a")
            .arg("copy")
            .arg(output_path)
            .output()
            .await?;

        if !output.status.success() {
            // Fallback: If container or stream copy has issues, re-encode audio to AAC
            let fallback_output = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(video_path)
                .arg("-i")
                .arg(audio_path)
                .arg("-c:v")
                .arg("copy")
                .arg("-c:a")
                .arg("aac")
                .arg(output_path)
                .output()
                .await?;

            if !fallback_output.status.success() {
                let err_msg = String::from_utf8_lossy(&fallback_output.stderr);
                return Err(DlpError::FFmpegError(format!(
                    "ffmpeg stream merge failed: {}",
                    err_msg.lines().last().unwrap_or("Unknown error")
                )));
            }
        }

        Ok(())
    }

    /// Extracts audio track and converts to target format (mp3, m4a, flac, wav, opus)
    pub async fn extract_audio(
        input_path: &Path,
        output_path: &Path,
        audio_format: &str,
    ) -> Result<()> {
        if !Self::is_available().await {
            return Err(DlpError::FFmpegError(
                "ffmpeg executable not found in PATH.".into(),
            ));
        }

        println!("[ExtractAudio] Destination: {}", output_path.display());

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-i").arg(input_path).arg("-vn");

        match audio_format {
            "mp3" => {
                cmd.arg("-c:a").arg("libmp3lame").arg("-q:a").arg("0");
            }
            "m4a" | "aac" => {
                cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");
            }
            "opus" => {
                cmd.arg("-c:a").arg("libopus").arg("-b:a").arg("160k");
            }
            "flac" => {
                cmd.arg("-c:a").arg("flac");
            }
            "wav" => {
                cmd.arg("-c:a").arg("pcm_s16le");
            }
            _ => {
                cmd.arg("-c:a").arg("copy");
            }
        }

        cmd.arg(output_path);

        let output = cmd.output().await?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DlpError::FFmpegError(format!(
                "ffmpeg audio extraction to {} failed: {}",
                audio_format,
                err_msg.lines().last().unwrap_or("Unknown error")
            )));
        }

        Ok(())
    }

    /// Embeds subtitle track into MP4 or MKV container
    pub async fn embed_subtitles(
        media_path: &Path,
        sub_path: &Path,
        lang: &str,
        output_path: &Path,
    ) -> Result<()> {
        if !Self::is_available().await {
            return Ok(());
        }

        println!(
            "[EmbedSubtitle] Embedding {} subtitles into {}",
            lang,
            output_path.display()
        );

        let ext = output_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mp4");
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-i")
            .arg(media_path)
            .arg("-i")
            .arg(sub_path)
            .arg("-c")
            .arg("copy");

        if ext == "mp4" {
            cmd.arg("-c:s").arg("mov_text");
        }

        cmd.arg("-metadata:s:s:0")
            .arg(format!("language={}", lang))
            .arg(output_path);

        let output = cmd.output().await?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DlpError::FFmpegError(format!(
                "ffmpeg subtitle embedding failed: {}",
                err_msg.lines().last().unwrap_or("Unknown error")
            )));
        }

        Ok(())
    }

    /// Embeds thumbnail as cover art into MP4, MKV, or MP3
    pub async fn embed_thumbnail(
        media_path: &Path,
        thumb_path: &Path,
        output_path: &Path,
    ) -> Result<()> {
        if !Self::is_available().await {
            return Ok(());
        }

        println!(
            "[EmbedThumbnail] Embedding thumbnail into {}",
            output_path.display()
        );

        let ext = output_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mp4");
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-i")
            .arg(media_path)
            .arg("-i")
            .arg(thumb_path);

        if ext == "mp3" {
            cmd.arg("-map")
                .arg("0:0")
                .arg("-map")
                .arg("1:0")
                .arg("-c")
                .arg("copy")
                .arg("-id3v2_version")
                .arg("3")
                .arg("-metadata:s:v")
                .arg("title=Album cover")
                .arg("-metadata:s:v")
                .arg("comment=Cover (front)");
        } else {
            cmd.arg("-map")
                .arg("0")
                .arg("-map")
                .arg("1")
                .arg("-c")
                .arg("copy")
                .arg("-disposition:v:1")
                .arg("attached_pic");
        }

        cmd.arg(output_path);

        let output = cmd.output().await?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DlpError::FFmpegError(format!(
                "ffmpeg thumbnail embedding failed: {}",
                err_msg.lines().last().unwrap_or("Unknown error")
            )));
        }

        Ok(())
    }
}
