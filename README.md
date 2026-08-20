# rustdlp

A high-performance, modular, feature-based YouTube downloader in Rust, ported and architected from Python `yt-dlp`.

## Features

- **Modular Architecture**: Feature-separated layers (`core`, `models`, `extractor`, `downloader`, `postprocessor`, `cli`).
- **Multi-Client Innertube Engine**: Extracts formats across multiple YouTube clients (`visionos`, `android_vr`, `android`, `ios`, `web`) for complete stream coverage (144p up to 4K/2160p 60fps AV1/VP9, Opus 160kbps, AAC 128kbps, and progressive formats).
- **High-Performance Chunked Range Downloader**: Downloads stream fragments using HTTP range chunks (5 MiB) to bypass YouTube playback rate throttling and maximize download bandwidth. Supports resume capability with `.part` files.
- **Intelligent Format Selection**: Supports `best`, `bestvideo+bestaudio`, `bestaudio`, resolution filters (`1080p`, `720p`), container filters (`mp4`, `webm`), and itags (`137+140`, `313+251`, `18`).
- **FFmpeg Multiplexing & Audio Extraction**: Automatically muxes separate video and audio streams into `.mp4` or `.webm` without re-encoding, and extracts audio to `mp3`, `m4a`, `opus`, `flac`, `wav` (`-x`).
- **Metadata & Thumbnails**: `-F` format tables, `-j` JSON dumps, `--write-info-json`, `--write-thumbnail`.

## Project Structure

```
rustdlp/
├── Cargo.toml
├── src/
│   ├── main.rs                   # CLI entrypoint & workflow orchestration
│   ├── lib.rs                    # Re-exports public library components
│   │
│   ├── core/                     # Primitives, domain types, utilities, errors
│   │   ├── error.rs              # Strongly-typed error enum (DlpError)
│   │   ├── types.rs              # VideoId, FormatId, MediaType, Protocol, Resolution
│   │   ├── utils.rs              # Size/Duration formatters, path templating, filename sanitizer
│   │   └── mod.rs
│   │
│   ├── models/                   # Strongly-typed Serde data models
│   │   ├── innertube.rs          # Raw YouTube Innertube API player response
│   │   ├── format.rs             # StreamFormat parsed representation
│   │   ├── video.rs              # VideoMetadata
│   │   ├── selector.rs           # FormatSelector expression evaluator
│   │   └── mod.rs
│   │
│   ├── extractor/                # Video extraction engine
│   │   ├── traits.rs             # Extractor async trait
│   │   └── youtube/
│   │       ├── client.rs         # Innertube API client & multi-client support
│   │       ├── url.rs            # YouTube URL matcher (watch, shorts, embed, youtu.be)
│   │       ├── parser.rs         # Format mime parser, itag table, codecs
│   │       ├── cipher.rs         # Signature cipher & URL resolver
│   │       ├── extractor.rs      # YoutubeExtractor orchestrator & deduplicator
│   │       └── mod.rs
│   │
│   ├── downloader/               # Download engines
│   │   ├── traits.rs             # StreamDownloader trait
│   │   ├── progress.rs           # Indicatif progress bar with speed & ETA
│   │   ├── http.rs               # Chunked HTTP range downloader with resume support
│   │   ├── single.rs             # Single stream downloader
│   │   ├── adaptive.rs           # Concurrent multi-stream downloader & merger
│   │   └── mod.rs
│   │
│   ├── postprocessor/            # Media processing
│   │   ├── ffmpeg.rs             # FFmpeg stream merger & audio converter
│   │   ├── metadata.rs           # JSON writer & thumbnail downloader
│   │   └── mod.rs
│   │
│   └── cli/                      # Command line interface
│       ├── args.rs               # Clap CLI argument specifications
│       ├── output.rs             # Colored terminal format tables & info logs
│       └── mod.rs
```

## Quick Start

### Build

```bash
cargo build --release
```

### Usage Examples

```bash
# 1. List all available formats
./target/release/rustdlp -F "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 2. Download best video + best audio merged into MP4
./target/release/rustdlp "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 3. Download specific resolution (e.g. 720p or 1080p)
./target/release/rustdlp -f "720p" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 4. Extract audio and convert to MP3
./target/release/rustdlp -x --audio-format mp3 "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 5. Save metadata and thumbnail
./target/release/rustdlp --write-info-json --write-thumbnail "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 6. Dump JSON metadata to stdout
./target/release/rustdlp -j "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```
