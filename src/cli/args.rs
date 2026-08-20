use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "rustdlp",
    author = "Antigravity",
    version = "0.1.0",
    about = "High-performance modular YouTube downloader in Rust inspired by yt-dlp"
)]
pub struct CliArgs {
    /// YouTube video or playlist URL or ID to download
    #[arg(required = true)]
    pub url: String,

    /// List all available formats and exit
    #[arg(short = 'F', long = "list-formats")]
    pub list_formats: bool,

    /// Video format code / selection expression (e.g. "best", "bestvideo+bestaudio", "137+140", "1080p", "mp4")
    #[arg(
        short = 'f',
        long = "format",
        default_value = "bestvideo+bestaudio/best"
    )]
    pub format: String,

    /// Output filename template (e.g. "%(title)s [%(id)s].%(ext)s")
    #[arg(
        short = 'o',
        long = "output",
        default_value = "%(title)s [%(id)s].%(ext)s"
    )]
    pub output: String,

    /// Convert video to audio-only file
    #[arg(short = 'x', long = "extract-audio")]
    pub extract_audio: bool,

    /// Specify audio format when extracting audio ("mp3", "m4a", "opus", "flac", "wav")
    #[arg(long = "audio-format", default_value = "mp3")]
    pub audio_format: String,

    /// Dump video or playlist metadata as JSON to stdout and exit
    #[arg(short = 'j', long = "dump-json")]
    pub dump_json: bool,

    /// Write video metadata to a .info.json file
    #[arg(long = "write-info-json")]
    pub write_info_json: bool,

    /// Write thumbnail image to disk
    #[arg(long = "write-thumbnail")]
    pub write_thumbnail: bool,

    /// Embed thumbnail into video/audio file as cover art
    #[arg(long = "embed-thumbnail")]
    pub embed_thumbnail: bool,

    /// List available subtitles and exit
    #[arg(long = "list-subs")]
    pub list_subs: bool,

    /// Download subtitles
    #[arg(long = "write-subs")]
    pub write_subs: bool,

    /// Download automatically generated subtitles
    #[arg(long = "write-auto-subs")]
    pub write_auto_subs: bool,

    /// Languages of subtitles to download (comma-separated, e.g. "en,es" or "all")
    #[arg(long = "sub-lang", default_value = "en")]
    pub sub_lang: String,

    /// Subtitle format ("vtt" or "srt")
    #[arg(long = "sub-format", default_value = "vtt")]
    pub sub_format: String,

    /// Embed subtitle track into video file
    #[arg(long = "embed-subs")]
    pub embed_subs: bool,

    /// Use HTTP/HTTPS/SOCKS5 proxy (e.g. "http://127.0.0.1:8080" or "socks5://127.0.0.1:1080")
    #[arg(long = "proxy")]
    pub proxy: Option<String>,

    /// Netscape formatted cookie file path
    #[arg(long = "cookies")]
    pub cookies: Option<PathBuf>,

    /// Playlist items to download (e.g. "1,2,5-10")
    #[arg(long = "playlist-items")]
    pub playlist_items: Option<String>,

    /// Playlist start index (1-based)
    #[arg(long = "playlist-start")]
    pub playlist_start: Option<usize>,

    /// Playlist end index (1-based)
    #[arg(long = "playlist-end")]
    pub playlist_end: Option<usize>,

    /// Download the playlist, if the URL refers to a video and a playlist
    #[arg(long = "yes-playlist")]
    pub yes_playlist: bool,

    /// Download only the video, if the URL refers to a video and a playlist
    #[arg(long = "no-playlist")]
    pub no_playlist: bool,

    /// Keep intermediate video and audio stream fragments after muxing
    #[arg(short = 'k', long = "keep-video")]
    pub keep_video: bool,

    /// Quiet mode, suppress non-error messages
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}
