use crate::core::error::{DlpError, Result};
use crate::models::format::StreamFormat;
use crate::models::video::VideoMetadata;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum SelectedFormat {
    Single(StreamFormat),
    Dual {
        video: StreamFormat,
        audio: StreamFormat,
    },
}

impl SelectedFormat {
    pub fn is_dual(&self) -> bool {
        matches!(self, SelectedFormat::Dual { .. })
    }

    pub fn total_filesize(&self) -> Option<u64> {
        match self {
            SelectedFormat::Single(f) => f.effective_filesize(),
            SelectedFormat::Dual { video, audio } => {
                match (video.effective_filesize(), audio.effective_filesize()) {
                    (Some(v), Some(a)) => Some(v + a),
                    (Some(v), None) => Some(v),
                    (None, Some(a)) => Some(a),
                    (None, None) => None,
                }
            }
        }
    }
}

pub struct FormatSelector;

impl FormatSelector {
    /// Evaluates a format selection expression (e.g., "best", "bestvideo+bestaudio", "137+140", "mp4", "1080p")
    pub fn select(metadata: &VideoMetadata, expr: Option<&str>) -> Result<SelectedFormat> {
        let raw_expr = expr.unwrap_or("bestvideo+bestaudio/best").trim();

        // Check for slash fallback: "bestvideo+bestaudio/best"
        for sub_expr in raw_expr.split('/') {
            let trimmed = sub_expr.trim();
            if let Ok(selected) = Self::select_single_expr(metadata, trimmed) {
                return Ok(selected);
            }
        }

        Err(DlpError::NoFormatFound(raw_expr.to_string()))
    }

    fn select_single_expr(metadata: &VideoMetadata, expr: &str) -> Result<SelectedFormat> {
        let formats = &metadata.formats;
        if formats.is_empty() {
            return Err(DlpError::NoFormatFound(
                "No formats available for video".into(),
            ));
        }

        // Check for compound expression (e.g., "bestvideo+bestaudio" or "137+140")
        if let Some((v_expr, a_expr)) = expr.split_once('+') {
            let v_format = Self::find_video_format(formats, v_expr.trim())?;
            let a_format = Self::find_audio_format(formats, a_expr.trim())?;
            return Ok(SelectedFormat::Dual {
                video: v_format,
                audio: a_format,
            });
        }

        // Single format selection
        match expr {
            "best" | "" => {
                if let (Ok(v), Ok(a)) = (Self::best_video(formats), Self::best_audio(formats)) {
                    Ok(SelectedFormat::Dual { video: v, audio: a })
                } else if let Some(comb) = Self::best_combined(formats) {
                    Ok(SelectedFormat::Single(comb))
                } else if let Ok(v) = Self::best_video(formats) {
                    Ok(SelectedFormat::Single(v))
                } else if let Ok(a) = Self::best_audio(formats) {
                    Ok(SelectedFormat::Single(a))
                } else {
                    Err(DlpError::NoFormatFound("best".into()))
                }
            }
            "bestvideo" => Self::best_video(formats).map(SelectedFormat::Single),
            "bestaudio" => Self::best_audio(formats).map(SelectedFormat::Single),
            "worst" => Self::worst_video(formats).map(SelectedFormat::Single),
            "worstaudio" => Self::worst_audio(formats).map(SelectedFormat::Single),
            "mp4" => {
                let v = formats
                    .iter()
                    .filter(|f| f.ext == "mp4" && f.is_video_only())
                    .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()));
                let a = Self::best_audio_ext(formats, "m4a").or_else(|_| Self::best_audio(formats));
                match (v, a) {
                    (Some(v_fmt), Ok(a_fmt)) => Ok(SelectedFormat::Dual {
                        video: v_fmt.clone(),
                        audio: a_fmt,
                    }),
                    _ => {
                        if let Some(f) = formats
                            .iter()
                            .filter(|f| f.ext == "mp4" && f.is_combined())
                            .max_by_key(|f| f.height())
                        {
                            Ok(SelectedFormat::Single(f.clone()))
                        } else {
                            Err(DlpError::NoFormatFound("mp4".into()))
                        }
                    }
                }
            }
            "webm" => {
                let v = formats
                    .iter()
                    .filter(|f| f.ext == "webm" && f.is_video_only())
                    .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()));
                let a =
                    Self::best_audio_ext(formats, "webm").or_else(|_| Self::best_audio(formats));
                match (v, a) {
                    (Some(v_fmt), Ok(a_fmt)) => Ok(SelectedFormat::Dual {
                        video: v_fmt.clone(),
                        audio: a_fmt,
                    }),
                    _ => {
                        if let Some(f) = formats
                            .iter()
                            .filter(|f| f.ext == "webm" && f.is_combined())
                            .max_by_key(|f| f.height())
                        {
                            Ok(SelectedFormat::Single(f.clone()))
                        } else {
                            Err(DlpError::NoFormatFound("webm".into()))
                        }
                    }
                }
            }
            itag_or_res => {
                if let Some(f) = formats.iter().find(|f| {
                    f.format_id.as_str() == itag_or_res || f.itag.to_string() == itag_or_res
                }) {
                    return Ok(SelectedFormat::Single(f.clone()));
                }

                let target_height: Option<u32> = itag_or_res.trim_end_matches('p').parse().ok();
                if let Some(h) = target_height
                    && let Some(v) = formats
                        .iter()
                        .filter(|f| f.height() == h && f.is_video())
                        .max_by_key(|f| (f.effective_fps(), f.effective_bitrate()))
                {
                    if v.is_combined() {
                        return Ok(SelectedFormat::Single(v.clone()));
                    } else if let Ok(a) = Self::best_audio(formats) {
                        return Ok(SelectedFormat::Dual {
                            video: v.clone(),
                            audio: a,
                        });
                    } else {
                        return Ok(SelectedFormat::Single(v.clone()));
                    }
                }

                Err(DlpError::NoFormatFound(expr.to_string()))
            }
        }
    }

    fn find_video_format(formats: &[StreamFormat], expr: &str) -> Result<StreamFormat> {
        match expr {
            "bestvideo" | "best" => Self::best_video(formats),
            "worstvideo" | "worst" => Self::worst_video(formats),
            other => {
                if let Some(f) = formats
                    .iter()
                    .find(|f| f.format_id.as_str() == other || f.itag.to_string() == other)
                {
                    Ok(f.clone())
                } else if let Ok(h) = other.trim_end_matches('p').parse::<u32>() {
                    formats
                        .iter()
                        .filter(|f| f.height() == h && f.is_video())
                        .max_by_key(|f| (f.effective_fps(), f.effective_bitrate()))
                        .cloned()
                        .ok_or_else(|| DlpError::NoFormatFound(format!("Video resolution {}p", h)))
                } else {
                    Err(DlpError::NoFormatFound(format!(
                        "Video selector '{}'",
                        other
                    )))
                }
            }
        }
    }

    fn find_audio_format(formats: &[StreamFormat], expr: &str) -> Result<StreamFormat> {
        match expr {
            "bestaudio" | "best" => Self::best_audio(formats),
            "worstaudio" | "worst" => Self::worst_audio(formats),
            "m4a" => Self::best_audio_ext(formats, "m4a"),
            "opus" | "webm" => Self::best_audio_ext(formats, "webm"),
            other => {
                if let Some(f) = formats
                    .iter()
                    .find(|f| f.format_id.as_str() == other || f.itag.to_string() == other)
                {
                    Ok(f.clone())
                } else {
                    Err(DlpError::NoFormatFound(format!(
                        "Audio selector '{}'",
                        other
                    )))
                }
            }
        }
    }

    pub fn best_combined(formats: &[StreamFormat]) -> Option<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_combined() && f.height() > 0)
            .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()))
            .cloned()
    }

    pub fn best_video(formats: &[StreamFormat]) -> Result<StreamFormat> {
        if let Some(f) = formats
            .iter()
            .filter(|f| f.is_video_only())
            .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()))
        {
            return Ok(f.clone());
        }
        formats
            .iter()
            .filter(|f| f.is_video())
            .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("bestvideo".into()))
    }

    pub fn worst_video(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_video() && f.height() > 0)
            .min_by_key(|f| (f.height(), f.effective_bitrate()))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("worstvideo".into()))
    }

    pub fn best_audio(formats: &[StreamFormat]) -> Result<StreamFormat> {
        if let Some(f) = formats
            .iter()
            .filter(|f| f.is_audio_only())
            .max_by_key(|f| (f.effective_bitrate(), f.audio_sample_rate.unwrap_or(0)))
        {
            return Ok(f.clone());
        }
        formats
            .iter()
            .filter(|f| f.is_audio())
            .max_by_key(|f| (f.effective_bitrate(), f.audio_sample_rate.unwrap_or(0)))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("bestaudio".into()))
    }

    pub fn worst_audio(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_audio() && f.effective_bitrate() > 0)
            .min_by_key(|f| f.effective_bitrate())
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("worstaudio".into()))
    }

    pub fn best_audio_ext(formats: &[StreamFormat], ext: &str) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| {
                f.is_audio()
                    && (f.ext == ext
                        || (ext == "m4a" && f.ext == "mp4")
                        || (ext == "opus" && f.ext == "webm"))
            })
            .max_by_key(|f| (f.effective_bitrate(), f.audio_sample_rate.unwrap_or(0)))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound(format!("bestaudio [{}]", ext)))
    }
}
