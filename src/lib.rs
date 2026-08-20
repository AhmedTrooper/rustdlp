pub mod cli;
pub mod core;
pub mod downloader;
pub mod extractor;
pub mod models;
pub mod postprocessor;

pub use core::error::{DlpError, Result};
pub use core::types::{FormatId, MediaType, Protocol, Resolution, VideoId};
pub use downloader::{AdaptiveDownloader, HttpDownloader, SingleStreamDownloader, StreamDownloader};
pub use extractor::traits::Extractor;
pub use extractor::youtube::YoutubeExtractor;
pub use models::format::StreamFormat;
pub use models::selector::{FormatSelector, SelectedFormat};
pub use models::video::VideoMetadata;
pub use postprocessor::ffmpeg::FFmpeg;
pub use postprocessor::metadata::MetadataWriter;
