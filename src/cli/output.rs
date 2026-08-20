use crate::core::utils::{format_bitrate, format_bytes, format_duration};
use crate::models::video::VideoMetadata;
use colored::*;

pub fn print_video_info(meta: &VideoMetadata) {
    println!(
        "{} [{}] {}: {}",
        "[youtube]".green().bold(),
        meta.id.as_str().cyan(),
        "Extracting URL".bold(),
        meta.webpage_url
    );
    println!(
        "{} [{}] {}: {}",
        "[youtube]".green().bold(),
        meta.id.as_str().cyan(),
        "Title".bold(),
        meta.title
    );
    if let Some(dur) = meta.duration {
        println!(
            "{} [{}] {}: {} by {}",
            "[youtube]".green().bold(),
            meta.id.as_str().cyan(),
            "Duration".bold(),
            format_duration(dur),
            meta.uploader
        );
    }
}

pub fn print_subtitles_table(meta: &VideoMetadata) {
    if meta.subtitles.is_empty() {
        println!(
            "{} No subtitles available for {}",
            "[info]".blue().bold(),
            meta.id.as_str().cyan()
        );
        return;
    }

    println!(
        "{} Available subtitles for {}:",
        "[info]".blue().bold(),
        meta.id.as_str().cyan()
    );
    println!(
        "{:<10} {:<30} {}",
        "Language".bold(),
        "Name".bold(),
        "Type".bold()
    );
    println!("{}", "─".repeat(60).dimmed());

    for sub in &meta.subtitles {
        let kind = if sub.is_auto_generated {
            "auto-generated (ASR)".yellow()
        } else {
            "manual".green()
        };
        println!("{:<10} {:<30} {}", sub.language_code.cyan(), sub.name, kind);
    }
}

pub fn print_format_table(meta: &VideoMetadata) {
    println!(
        "{} Available formats for {}:",
        "[info]".blue().bold(),
        meta.id.as_str().cyan()
    );

    println!(
        "{:<5} {:<5} {:<12} {:<4} {:<2} │ {:>10} {:>7} {:<5} │ {:<18} {:<15} {}",
        "ID".bold(),
        "EXT".bold(),
        "RESOLUTION".bold(),
        "FPS".bold(),
        "CH".bold(),
        "FILESIZE".bold(),
        "TBR".bold(),
        "PROTO".bold(),
        "VCODEC".bold(),
        "ACODEC".bold(),
        "MORE INFO".bold(),
    );
    println!("{}", "─".repeat(105).dimmed());

    for f in &meta.formats {
        let id_str = f.itag.to_string();
        let ext_str = f.ext.as_str();

        let res_str = match (f.resolution.width, f.resolution.height) {
            (Some(w), Some(h)) => format!("{}x{}", w, h),
            (None, Some(h)) => format!("{}p", h),
            _ if f.is_audio_only() => "audio only".to_string(),
            _ => "unknown".to_string(),
        };

        let fps_str = f.fps.map(|fps| fps.to_string()).unwrap_or_default();
        let ch_str = f
            .audio_channels
            .map(|ch| ch.to_string())
            .unwrap_or_default();

        let size_str = match (f.filesize, f.filesize_approx) {
            (Some(sz), _) => format_bytes(sz),
            (None, Some(approx)) => format!("~{}", format_bytes(approx)),
            (None, None) => "unknown".to_string(),
        };

        let tbr_str = f.bitrate.map(format_bitrate).unwrap_or_default();
        let proto_str = f.protocol.to_string();

        let vcodec_str = f.vcodec.as_deref().unwrap_or(if f.is_audio_only() {
            "audio only"
        } else {
            "none"
        });
        let acodec_str = f.acodec.as_deref().unwrap_or(if f.is_video_only() {
            "video only"
        } else {
            "none"
        });

        let mut more_info = Vec::new();
        if let Some(ref ql) = f.quality_label {
            more_info.push(ql.as_str());
        }
        more_info.push(f.source_client.as_str());

        let info_str = more_info.join(", ");

        println!(
            "{:<5} {:<5} {:<12} {:<4} {:<2} │ {:>10} {:>7} {:<5} │ {:<18} {:<15} {}",
            id_str.yellow(),
            ext_str.green(),
            res_str,
            fps_str,
            ch_str,
            size_str,
            tbr_str,
            proto_str,
            vcodec_str,
            acodec_str,
            info_str.dimmed(),
        );
    }
}
