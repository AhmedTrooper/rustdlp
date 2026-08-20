use clap::Parser;
use colored::*;
use rustdlp::cli::{print_format_table, print_video_info, CliArgs};
use rustdlp::core::error::Result;
use rustdlp::core::utils::resolve_output_path;
use rustdlp::downloader::{AdaptiveDownloader, SingleStreamDownloader, StreamDownloader};
use rustdlp::extractor::{Extractor, YoutubeExtractor};
use rustdlp::models::{FormatSelector, SelectedFormat};
use rustdlp::postprocessor::{FFmpeg, MetadataWriter};

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    let extractor = YoutubeExtractor::new();
    let metadata = match extractor.extract(&args.url).await {
        Ok(meta) => meta,
        Err(err) => {
            eprintln!("{} {}", "ERROR:".red().bold(), err);
            std::process::exit(1);
        }
    };

    // Dump JSON mode (-j)
    if args.dump_json {
        let json_str = serde_json::to_string_pretty(&metadata)?;
        println!("{}", json_str);
        return Ok(());
    }

    if !args.quiet {
        print_video_info(&metadata);
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
        MetadataWriter::write_info_json(&metadata, &info_path).await?;
    }

    if args.write_thumbnail {
        let thumb_path = resolve_output_path(
            &args.output,
            &metadata.title,
            metadata.id.as_str(),
            &metadata.uploader,
            "jpg",
        );
        let http = reqwest::Client::new();
        MetadataWriter::download_thumbnail(&http, &metadata, &thumb_path).await?;
    }

    // List formats mode (-F)
    if args.list_formats {
        print_format_table(&metadata);
        return Ok(());
    }

    // Audio extraction mode (-x)
    if args.extract_audio {
        let audio_format = FormatSelector::select(&metadata, Some("bestaudio"))?;
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

        println!("{} Download completed: {}", "[download]".green().bold(), final_audio_path.display());
        return Ok(());
    }

    // Video download mode
    let selected = match FormatSelector::select(&metadata, Some(&args.format)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} {}", "ERROR:".red().bold(), e);
            std::process::exit(1);
        }
    };

    match selected {
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
            println!("{} Download completed: {}", "[download]".green().bold(), output_path.display());
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

            println!("{} Download completed: {}", "[download]".green().bold(), output_path.display());
        }
    }

    Ok(())
}
