use clap::Parser;
use colored::*;
use rustdlp::cli::{CliArgs, print_format_table, print_subtitles_table, print_video_info};
use rustdlp::core::config::DlpConfig;
use rustdlp::core::error::Result;
use rustdlp::core::utils::resolve_output_path;
use rustdlp::downloader::{AdaptiveDownloader, SingleStreamDownloader, StreamDownloader};
use rustdlp::extractor::youtube::subtitle::SubtitleDownloader;
use rustdlp::extractor::youtube::url::extract_playlist_id;
use rustdlp::extractor::{ExtractorRegistry, YoutubeExtractor, YoutubePlaylistExtractor};
use rustdlp::models::{FormatSelector, SelectedFormat, VideoMetadata};
use rustdlp::postprocessor::{FFmpeg, MetadataWriter};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    // 1. Initialize configuration with proxy & cookies
    let config = DlpConfig::new()
        .with_proxy(args.proxy.clone())
        .with_cookies(args.cookies.clone());

    let http_client = config.build_http_client()?;

    // 2. YouTube Playlist Routing
    let playlist_id = extract_playlist_id(&args.url);
    let is_playlist_request = playlist_id.is_some() && (args.yes_playlist || !args.no_playlist);

    if let (Some(ref pl_id), true) = (playlist_id, is_playlist_request)
        && !args.no_playlist
    {
        return process_playlist(&args, pl_id, &http_client).await;
    }

    // 3. Automated Multi-Site Extractor Dispatch via Registry
    let registry = ExtractorRegistry::new(http_client.clone());
    let metadata = match registry.extract(&args.url).await {
        Ok(meta) => meta,
        Err(err) => {
            eprintln!("{} {}", "ERROR:".red().bold(), err);
            std::process::exit(1);
        }
    };

    process_single_video(&args, &metadata, &http_client).await
}

async fn process_playlist(
    args: &CliArgs,
    playlist_id: &str,
    http_client: &reqwest::Client,
) -> Result<()> {
    println!(
        "{} [{}] Extracting playlist: {}",
        "[youtube:tab]".green().bold(),
        playlist_id.cyan(),
        args.url
    );

    let playlist_extractor = YoutubePlaylistExtractor::new(http_client.clone());
    let playlist_meta = playlist_extractor.extract_playlist(playlist_id).await?;

    println!(
        "{} [{}] Playlist {}: {} videos found",
        "[youtube:tab]".green().bold(),
        playlist_id.cyan(),
        playlist_meta.title.bold(),
        playlist_meta.video_count()
    );

    if args.dump_json {
        let json_str = serde_json::to_string_pretty(&playlist_meta)?;
        println!("{}", json_str);
        return Ok(());
    }

    // Filter playlist items based on --playlist-start, --playlist-end, --playlist-items
    let items_len = playlist_meta.items.len();
    let start_idx = args.playlist_start.unwrap_or(1).saturating_sub(1);
    let end_idx = args.playlist_end.unwrap_or(items_len).min(items_len);

    let mut filtered_indices: Vec<usize> = (start_idx..end_idx).collect();

    if let Some(ref items_expr) = args.playlist_items {
        filtered_indices = parse_playlist_items_expr(items_expr, items_len);
    }

    let total_to_download = filtered_indices.len();
    println!(
        "{} Downloading {} of {} videos in playlist",
        "[info]".blue().bold(),
        total_to_download,
        items_len
    );

    let extractor = YoutubeExtractor::with_http_client(http_client.clone());

    for (pos, &idx) in filtered_indices.iter().enumerate() {
        let item = &playlist_meta.items[idx];
        println!(
            "\n{} [{}/{}] Downloading video {} ({})",
            "[download]".green().bold(),
            pos + 1,
            total_to_download,
            item.id.as_str().cyan(),
            item.title.as_deref().unwrap_or("...")
        );

        match extractor.extract_by_id(&item.id).await {
            Ok(video_meta) => {
                if let Err(e) = process_single_video(args, &video_meta, http_client).await {
                    eprintln!(
                        "{} Failed to download {}: {}",
                        "WARNING:".yellow().bold(),
                        item.id,
                        e
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "{} Failed to extract {}: {}",
                    "WARNING:".yellow().bold(),
                    item.id,
                    e
                );
            }
        }
    }

    println!(
        "\n{} Playlist download complete!",
        "[download]".green().bold()
    );
    Ok(())
}

async fn process_single_video(
    args: &CliArgs,
    metadata: &VideoMetadata,
    http_client: &reqwest::Client,
) -> Result<()> {
    // Dump JSON mode (-j)
    if args.dump_json {
        let json_str = serde_json::to_string_pretty(metadata)?;
        println!("{}", json_str);
        return Ok(());
    }

    if !args.quiet {
        print_video_info(metadata);
    }

    // List subtitles mode (--list-subs)
    if args.list_subs {
        print_subtitles_table(metadata);
        return Ok(());
    }

    // Optional metadata and thumbnail downloads
    if args.write_info_json {
        let info_path = resolve_output_path(
            &args.output,
            &metadata.title,
            metadata.id.as_str(),
            &metadata.uploader,
            "info.json",
        );
        MetadataWriter::write_info_json(metadata, &info_path).await?;
    }

    let mut thumb_path: Option<PathBuf> = None;
    if args.write_thumbnail || args.embed_thumbnail {
        let path = resolve_output_path(
            &args.output,
            &metadata.title,
            metadata.id.as_str(),
            &metadata.uploader,
            "jpg",
        );
        MetadataWriter::download_thumbnail(http_client, metadata, &path).await?;
        thumb_path = Some(path);
    }

    // Subtitle downloads (--write-subs / --write-auto-subs)
    let mut downloaded_subs: Vec<(String, PathBuf)> = Vec::new();
    if args.write_subs || args.write_auto_subs || args.embed_subs {
        let sub_downloader = SubtitleDownloader::new(http_client.clone());
        let langs: Vec<&str> = args.sub_lang.split(',').map(|s| s.trim()).collect();
        let mut all_tracks = metadata.subtitles.clone();
        if let Some(base_track) = metadata.subtitles.first() {
            for &lang in &langs {
                if lang != "all" && !all_tracks.iter().any(|t| t.language_code == lang) {
                    let translated =
                        SubtitleDownloader::generate_translated_tracks(base_track, &[(lang, lang)]);
                    all_tracks.extend(translated);
                }
            }
        }

        for track in &all_tracks {
            if !args.write_auto_subs && track.is_auto_generated && !args.write_subs {
                continue;
            }
            if langs.contains(&"all") || langs.contains(&track.language_code.as_str()) {
                let sub_filename = format!("{}.{}", track.language_code, args.sub_format);
                let sub_path = resolve_output_path(
                    &args.output,
                    &metadata.title,
                    metadata.id.as_str(),
                    &metadata.uploader,
                    &sub_filename,
                );
                if sub_downloader
                    .download_subtitle(track, &args.sub_format, &sub_path)
                    .await
                    .is_ok()
                {
                    downloaded_subs.push((track.language_code.clone(), sub_path));
                }
            }
        }
    }

    // List formats mode (-F)
    if args.list_formats {
        print_format_table(metadata);
        return Ok(());
    }

    // Audio extraction mode (-x)
    if args.extract_audio {
        let audio_format = FormatSelector::select(metadata, Some("bestaudio"))?;
        let stream = match audio_format {
            SelectedFormat::Single(s) => s,
            SelectedFormat::Dual { audio, .. } => audio,
        };

        let temp_audio_path = resolve_output_path(
            &args.output,
            &metadata.title,
            metadata.id.as_str(),
            &metadata.uploader,
            &stream.ext,
        );

        let final_audio_path = resolve_output_path(
            &args.output,
            &metadata.title,
            metadata.id.as_str(),
            &metadata.uploader,
            &args.audio_format,
        );

        println!(
            "{} Selected format itag {} ({} {}kbps)",
            "[info]".blue().bold(),
            stream.itag,
            stream.ext,
            stream.effective_bitrate() / 1000
        );

        let downloader = SingleStreamDownloader::new();
        downloader.download(&stream, &temp_audio_path).await?;

        if args.audio_format != stream.ext {
            FFmpeg::extract_audio(&temp_audio_path, &final_audio_path, &args.audio_format).await?;
            if !args.keep_video {
                let _ = tokio::fs::remove_file(&temp_audio_path).await;
            }
        }

        if args.embed_thumbnail
            && let Some(ref tp) = thumb_path
        {
            let embedded_path =
                final_audio_path.with_extension(format!("embed.{}", args.audio_format));
            if FFmpeg::embed_thumbnail(&final_audio_path, tp, &embedded_path)
                .await
                .is_ok()
            {
                let _ = tokio::fs::rename(&embedded_path, &final_audio_path).await;
            }
        }

        println!(
            "{} Download completed: {}",
            "[download]".green().bold(),
            final_audio_path.display()
        );
        return Ok(());
    }

    // Video download mode
    let selected = match FormatSelector::select(metadata, Some(&args.format)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} {}", "ERROR:".red().bold(), e);
            std::process::exit(1);
        }
    };

    let final_media_path = match selected {
        SelectedFormat::Single(stream) => {
            println!(
                "{} Selected single stream itag {} ({})",
                "[info]".blue().bold(),
                stream.itag,
                stream.resolution
            );

            let output_path = resolve_output_path(
                &args.output,
                &metadata.title,
                metadata.id.as_str(),
                &metadata.uploader,
                &stream.ext,
            );

            let downloader = SingleStreamDownloader::new();
            downloader.download(&stream, &output_path).await?;
            output_path
        }
        SelectedFormat::Dual { video, audio } => {
            println!(
                "{} Selected video itag {} ({}) + audio itag {} ({}kbps)",
                "[info]".blue().bold(),
                video.itag,
                video.resolution,
                audio.itag,
                audio.effective_bitrate() / 1000
            );

            let final_ext = if video.ext == "webm" && audio.ext == "opus" {
                "webm"
            } else {
                "mp4"
            };

            let output_path = resolve_output_path(
                &args.output,
                &metadata.title,
                metadata.id.as_str(),
                &metadata.uploader,
                final_ext,
            );

            let adaptive = AdaptiveDownloader::new();
            adaptive
                .download_and_merge(&video, &audio, &output_path, args.keep_video)
                .await?;

            output_path
        }
    };

    // Post-processing: Embed subtitles if requested
    if args.embed_subs && !downloaded_subs.is_empty() {
        let (lang, sub_path) = &downloaded_subs[0];
        let sub_embedded_path = final_media_path.with_extension("sub.mp4");
        if FFmpeg::embed_subtitles(&final_media_path, sub_path, lang, &sub_embedded_path)
            .await
            .is_ok()
        {
            let _ = tokio::fs::rename(&sub_embedded_path, &final_media_path).await;
        }
    }

    // Post-processing: Embed thumbnail if requested
    if args.embed_thumbnail
        && let Some(ref tp) = thumb_path
    {
        let thumb_embedded_path = final_media_path.with_extension("thumb.mp4");
        if FFmpeg::embed_thumbnail(&final_media_path, tp, &thumb_embedded_path)
            .await
            .is_ok()
        {
            let _ = tokio::fs::rename(&thumb_embedded_path, &final_media_path).await;
        }
    }

    println!(
        "{} Download completed: {}",
        "[download]".green().bold(),
        final_media_path.display()
    );
    Ok(())
}

fn parse_playlist_items_expr(expr: &str, total: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    for part in expr.split(',') {
        let part = part.trim();
        if let Some((start_s, end_s)) = part.split_once('-') {
            let start: usize = start_s
                .trim()
                .parse::<usize>()
                .unwrap_or(1)
                .saturating_sub(1);
            let end: usize = end_s.trim().parse::<usize>().unwrap_or(total).min(total);
            for i in start..end {
                indices.push(i);
            }
        } else if let Ok(num) = part.parse::<usize>()
            && num >= 1
            && num <= total
        {
            indices.push(num - 1);
        }
    }
    indices
}
