use thiserror::Error;

pub type Result<T> = std::result::Result<T, DlpError>;

#[derive(Error, Debug)]
pub enum DlpError {
    #[error("Invalid URL or video ID: {0}")]
    InvalidUrl(String),

    #[error("Extraction failed: {0}")]
    ExtractionError(String),

    #[error("Video unavailable: {0}")]
    VideoUnavailable(String),

    #[error("Video is age restricted: {0}")]
    AgeRestricted(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No matching format found for selector: {0}")]
    NoFormatFound(String),

    #[error("Download error: {0}")]
    DownloadError(String),

    #[error("Post-processing error: {0}")]
    PostProcessor(String),

    #[error("Cipher deciphering error: {0}")]
    CipherError(String),

    #[error("FFmpeg not found or failed: {0}")]
    FFmpegError(String),

    #[error("{0}")]
    Custom(String),
}
