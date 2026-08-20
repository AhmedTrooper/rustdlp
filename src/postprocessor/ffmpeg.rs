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

        println!(
            "[ExtractAudio] Destination: {}",
            output_path.display()
        );

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-i")
            .arg(input_path)
            .arg("-vn");

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
}
