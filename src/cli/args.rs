use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "rustdlp",
    author = "Antigravity",
    version = "0.1.0",
    about = "High-performance modular YouTube downloader in Rust inspired by yt-dlp"
)]
pub struct CliArgs {
    /// YouTube video URL or ID to download
    #[arg(required = true)]
    pub url: String,

    /// List all available formats and exit
    #[arg(short = 'F', long = "list-formats")]
    pub list_formats: bool,

    /// Video format code / selection expression (e.g. "best", "bestvideo+bestaudio", "137+140", "1080p", "mp4")
    #[arg(short = 'f', long = "format", default_value = "bestvideo+bestaudio/best")]
    pub format: String,

    /// Output filename template (e.g. "%(title)s [%(id)s].%(ext)s")
    #[arg(short = 'o', long = "output", default_value = "%(title)s [%(id)s].%(ext)s")]
    pub output: String,

    /// Convert video to audio-only file
    #[arg(short = 'x', long = "extract-audio")]
    pub extract_audio: bool,

    /// Specify audio format when extracting audio ("mp3", "m4a", "opus", "flac", "wav")
    #[arg(long = "audio-format", default_value = "mp3")]
    pub audio_format: String,

    /// Dump video metadata as JSON to stdout and exit
    #[arg(short = 'j', long = "dump-json")]
    pub dump_json: bool,

    /// Write video metadata to a .info.json file
    #[arg(long = "write-info-json")]
    pub write_info_json: bool,

    /// Write thumbnail image to disk
    #[arg(long = "write-thumbnail")]
    pub write_thumbnail: bool,

    /// Keep intermediate video and audio stream fragments after muxing
    #[arg(short = 'k', long = "keep-video")]
    pub keep_video: bool,

    /// Quiet mode, suppress non-error messages
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}
