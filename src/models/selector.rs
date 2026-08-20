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

            if (v_format.is_combined() || a_format.is_combined())
                && v_format.format_id == a_format.format_id
            {
                return Ok(SelectedFormat::Single(v_format));
            }

            return Ok(SelectedFormat::Dual {
                video: v_format,
                audio: a_format,
            });
        }

        // Single format selection
        match expr {
            "best" | "" => {
                if let (Ok(v), Ok(a)) = (
                    Self::best_video_only(formats),
                    Self::best_audio_only(formats),
                ) {
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
            height_str if height_str.ends_with('p') => {
                let target_h: u32 = height_str[..height_str.len() - 1]
                    .parse()
                    .map_err(|_| DlpError::NoFormatFound(expr.to_string()))?;
                Self::find_by_resolution(formats, target_h)
            }
            itag_str if itag_str.parse::<u32>().is_ok() => {
                let itag: u32 = itag_str.parse().unwrap();
                formats
                    .iter()
                    .find(|f| f.itag == itag)
                    .cloned()
                    .map(SelectedFormat::Single)
                    .ok_or_else(|| DlpError::NoFormatFound(format!("itag {}", itag)))
            }
            id_str => formats
                .iter()
                .find(|f| f.format_id.as_str() == id_str)
                .cloned()
                .map(SelectedFormat::Single)
                .ok_or_else(|| DlpError::NoFormatFound(id_str.to_string())),
        }
    }

    fn find_video_format(formats: &[StreamFormat], expr: &str) -> Result<StreamFormat> {
        match expr {
            "bestvideo" | "best" => {
                Self::best_video_only(formats).or_else(|_| Self::best_video(formats))
            }
            "worstvideo" | "worst" => Self::worst_video(formats),
            h if h.ends_with('p') => {
                let target: u32 = h[..h.len() - 1]
                    .parse()
                    .map_err(|_| DlpError::NoFormatFound(expr.to_string()))?;
                formats
                    .iter()
                    .filter(|f| f.is_video() && f.height() <= target)
                    .max_by_key(|f| (f.height(), f.effective_bitrate()))
                    .cloned()
                    .ok_or_else(|| DlpError::NoFormatFound(expr.to_string()))
            }
            itag if itag.parse::<u32>().is_ok() => {
                let itag_num: u32 = itag.parse().unwrap();
                formats
                    .iter()
                    .find(|f| f.itag == itag_num && f.is_video())
                    .cloned()
                    .ok_or_else(|| DlpError::NoFormatFound(expr.to_string()))
            }
            id => formats
                .iter()
                .find(|f| f.format_id.as_str() == id && f.is_video())
                .cloned()
                .ok_or_else(|| DlpError::NoFormatFound(expr.to_string())),
        }
    }

    fn find_audio_format(formats: &[StreamFormat], expr: &str) -> Result<StreamFormat> {
        match expr {
            "bestaudio" | "best" => {
                Self::best_audio_only(formats).or_else(|_| Self::best_audio(formats))
            }
            "worstaudio" | "worst" => Self::worst_audio(formats),
            itag if itag.parse::<u32>().is_ok() => {
                let itag_num: u32 = itag.parse().unwrap();
                formats
                    .iter()
                    .find(|f| f.itag == itag_num && f.is_audio())
                    .cloned()
                    .ok_or_else(|| DlpError::NoFormatFound(expr.to_string()))
            }
            id => formats
                .iter()
                .find(|f| f.format_id.as_str() == id && f.is_audio())
                .cloned()
                .ok_or_else(|| DlpError::NoFormatFound(expr.to_string())),
        }
    }

    pub fn best_video_only(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_video_only())
            .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("best video only".into()))
    }

    pub fn best_audio_only(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_audio_only())
            .max_by_key(|f| f.effective_bitrate())
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("best audio only".into()))
    }

    pub fn best_video(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_video())
            .max_by_key(|f| (f.height(), f.effective_fps(), f.effective_bitrate()))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("best video".into()))
    }

    pub fn best_audio(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_audio())
            .max_by_key(|f| f.effective_bitrate())
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("best audio".into()))
    }

    pub fn worst_video(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_video())
            .min_by_key(|f| (f.height(), f.effective_bitrate()))
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("worst video".into()))
    }

    pub fn worst_audio(formats: &[StreamFormat]) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_audio())
            .min_by_key(|f| f.effective_bitrate())
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound("worst audio".into()))
    }

    pub fn best_combined(formats: &[StreamFormat]) -> Option<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_combined())
            .max_by_key(|f| (f.height(), f.effective_bitrate()))
            .cloned()
    }

    fn best_audio_ext(formats: &[StreamFormat], ext: &str) -> Result<StreamFormat> {
        formats
            .iter()
            .filter(|f| f.is_audio_only() && f.ext == ext)
            .max_by_key(|f| f.effective_bitrate())
            .cloned()
            .ok_or_else(|| DlpError::NoFormatFound(format!("best {} audio", ext)))
    }

    fn find_by_resolution(formats: &[StreamFormat], target_height: u32) -> Result<SelectedFormat> {
        if let Some(v) = formats
            .iter()
            .filter(|f| f.is_video_only() && f.height() <= target_height)
            .max_by_key(|f| f.height())
        {
            let a = Self::best_audio(formats)?;
            return Ok(SelectedFormat::Dual {
                video: v.clone(),
                audio: a,
            });
        }

        if let Some(c) = formats
            .iter()
            .filter(|f| f.is_combined() && f.height() <= target_height)
            .max_by_key(|f| f.height())
        {
            return Ok(SelectedFormat::Single(c.clone()));
        }

        Err(DlpError::NoFormatFound(format!("{}p", target_height)))
    }
}
