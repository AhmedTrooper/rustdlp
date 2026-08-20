pub mod cli;
pub mod core;
pub mod downloader;
pub mod extractor;
pub mod models;
pub mod postprocessor;

pub use core::config::DlpConfig;
pub use core::error::{DlpError, Result};
pub use core::types::{FormatId, MediaType, Protocol, Resolution, VideoId};
pub use downloader::{
    AdaptiveDownloader, HlsDownloader, HttpDownloader, SingleStreamDownloader, StreamDownloader,
};
pub use extractor::traits::Extractor;
pub use extractor::youtube::{SubtitleDownloader, YoutubeExtractor, YoutubePlaylistExtractor};
pub use models::format::StreamFormat;
pub use models::playlist::{PlaylistItem, PlaylistMetadata};
pub use models::selector::{FormatSelector, SelectedFormat};
pub use models::subtitle::{SubtitleConverter, SubtitleTrack};
pub use models::video::VideoMetadata;
pub use postprocessor::ffmpeg::FFmpeg;
pub use postprocessor::metadata::MetadataWriter;
