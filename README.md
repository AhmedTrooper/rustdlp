# rustdlp

A high-performance, modular, feature-complete YouTube downloader in Rust ported from Python `yt-dlp`.

## Features

- **Modular Feature Architecture**: Structured layers (`core`, `models`, `extractor`, `downloader`, `postprocessor`, `cli`).
- **Multi-Client Innertube Engine**: Extracts formats across multiple YouTube client contexts (`VisionOS`, `AndroidVR`, `Android`, `iOS`, `Web`) to aggregate all available resolutions from 144p up to 4K 2160p 60fps AV1/VP9, Opus 160kbps, AAC 128kbps, and progressive formats.
- **Anti-Throttling Chunked Downloader**: 5 MiB chunked HTTP range streaming to bypass YouTube playback rate throttling and maximize download bandwidth. Full resume capability via `.part` files.
- **Playlist & Multi-Video Support**: Extracts and downloads full YouTube playlists (`--playlist-items`, `--playlist-start`, `--playlist-end`, `--yes-playlist`).
- **Subtitles & Closed Captions**: Multi-language subtitle extraction, TimedText-to-SRT and TimedText-to-VTT conversion (`--list-subs`, `--write-subs`, `--write-auto-subs`, `--sub-lang`, `--sub-format`, `--embed-subs`).
- **FFmpeg Multiplexing & Post-Processing**: Lossless video + audio muxing into `.mp4`/`.webm`, audio extraction to MP3/M4A/Opus/FLAC/WAV (`-x`), and thumbnail/subtitle embedding (`--embed-thumbnail`, `--embed-subs`).
- **Proxy & Cookie Authentication**: Support for HTTP/HTTPS/SOCKS5 proxies (`--proxy`) and Netscape cookie files (`--cookies`) for authenticated and age-restricted downloads.
- **Metadata & Inspection**: Format tables (`-F`), JSON dumps (`-j`), `--write-info-json`, and `--write-thumbnail`.

## Quick Start

### Build

```bash
cargo build --release
```

### Usage Examples

```bash
# 1. List available formats
./target/release/rustdlp -F "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 2. Download best video + best audio merged into MP4
./target/release/rustdlp "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 3. Download specific resolution (e.g. 720p or 1080p)
./target/release/rustdlp -f "1080p" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 4. Extract audio and convert to MP3
./target/release/rustdlp -x --audio-format mp3 -o "%(title)s.%(ext)s" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 5. List and download subtitles (SRT / VTT)
./target/release/rustdlp --list-subs "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
./target/release/rustdlp --write-subs --sub-lang en --sub-format srt "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 6. Download items from a playlist
./target/release/rustdlp --playlist-items 1-5 "https://www.youtube.com/playlist?list=PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj"

# 7. Use proxy or cookies
./target/release/rustdlp --proxy "socks5://127.0.0.1:1080" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
./target/release/rustdlp --cookies cookies.txt "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```
