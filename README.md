# rustdlp

A high-performance, modular, feature-complete video downloader in Rust ported from Python `yt-dlp`.

## Supported Platforms

- **YouTube**: Multi-client Innertube engine (144p to 4K 2160p 60fps AV1/VP9/H264, Opus 160kbps, AAC 128kbps, full playlists, timedtext captions to SRT/VTT, HLS live streams).
- **Facebook**: Watch videos, Reels, Mobile URLs, Posts, Groups, DASH MPD manifests (1080p, 720p, 540p, 360p, audio only), and Progressive HD/SD MP4 streams.

## Features

- **Modular Architecture**: Feature-separated layers (`core`, `models`, `extractor`, `downloader`, `postprocessor`, `cli`).
- **Multi-Client & Multi-Platform Engine**: Comprehensive format extraction across platforms and client contexts.
- **Anti-Throttling Chunked Downloader**: 5 MiB chunked HTTP range streaming to bypass bandwidth rate throttling. Full resume capability via `.part` files.
- **Playlist & Multi-Video Support**: Extracts and downloads full playlists (`--playlist-items`, `--playlist-start`, `--playlist-end`, `--yes-playlist`).
- **Subtitles & Closed Captions**: Multi-language subtitle extraction, TimedText-to-SRT and TimedText-to-VTT conversion (`--list-subs`, `--write-subs`, `--write-auto-subs`, `--sub-lang`, `--sub-format`, `--embed-subs`).
- **FFmpeg Multiplexing & Post-Processing**: Lossless video + audio muxing into `.mp4`/`.webm`, audio extraction to MP3/M4A/Opus/FLAC/WAV (`-x`), and thumbnail/subtitle embedding (`--embed-thumbnail`, `--embed-subs`).
- **Proxy & Cookie Authentication**: Support for HTTP/HTTPS/SOCKS5 proxies (`--proxy`) and Netscape cookie files (`--cookies`) for authenticated downloads.
- **Metadata & Inspection**: Format tables (`-F`), JSON dumps (`-j`), `--write-info-json`, and `--write-thumbnail`.

## Quick Start

### Build

```bash
cargo build --release
```

### Usage Examples

```bash
# 1. YouTube: List formats
./target/release/rustdlp -F "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 2. YouTube: Download 1080p video + best audio merged
./target/release/rustdlp -f "1080p" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 3. YouTube: Extract audio to MP3 with embedded cover art
./target/release/rustdlp -x --audio-format mp3 --embed-thumbnail -o "%(title)s.%(ext)s" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 4. YouTube: Download subtitles (SRT / VTT)
./target/release/rustdlp --write-subs --sub-lang en --sub-format srt "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 5. Facebook: List available formats
./target/release/rustdlp -F "https://www.facebook.com/watch/?v=10154383743583686"

# 6. Facebook: Download HD video (720p/1080p)
./target/release/rustdlp -f "720p" "https://www.facebook.com/watch/?v=10154383743583686"

# 7. Facebook: Extract audio to MP3
./target/release/rustdlp -x --audio-format mp3 "https://www.facebook.com/watch/?v=10154383743583686"
```
