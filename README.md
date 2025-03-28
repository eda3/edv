# edv - CLI-based Video Editing Tool Written in Rust

An efficient and lightweight CLI-based video editing tool. No GUI required, easily integrates with scripts and pipelines.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.59%2B-orange.svg)

## Overview

edv is a high-performance video editing tool that works from the command line. Based on FFmpeg, it provides a user-friendly interface with rich features. Ideal for batch processing and automation, it runs efficiently even in resource-limited environments.

### Key Features

- 🎬 **Basic Video Editing** - Trimming, cutting, concatenation, speed adjustment
- 🌈 **Filters and Effects** - Color adjustment, filter application, transition effects
- 🔊 **Audio Processing** - Volume adjustment, fade in/out, extraction/replacement
- 📦 **Format Conversion** - Conversion to various formats, resolution changes, encoding settings
- 💬 **Subtitle Processing** - Adding, editing, and burning subtitles
- ⏱️ **Timeline Editing** - Multiple track support, keyframe effect control
- 🔄 **Batch Processing** - Bulk processing of multiple files, template application
- 📝 **Project Management** - Edit history with Undo/Redo, saving and loading project files

## Installation

### Prerequisites

- Rust 1.59 or later
- FFmpeg 4.2 or later

### Install from crates.io

```bash
cargo install edv
```

### Build from Source

```bash
git clone https://github.com/yourusername/edv.git
cd edv
cargo build --release
```

The executable will be generated at `target/release/edv`.

## Usage Examples

### Basic Video Editing

```bash
# Trim video (extract from 01:30 to 03:45)
edv trim --start 01:30 --end 03:45 input.mp4 -o output.mp4

# Concatenate multiple videos
edv concat video1.mp4 video2.mp4 video3.mp4 -o combined.mp4

# Cut sections (remove ranges 00:30-01:45 and 02:15-03:30)
edv cut --ranges 00:30-01:45,02:15-03:30 input.mp4 -o output.mp4

# Speed adjustment (0.5x slow, 2x fast)
edv speed --factor 0.5 input.mp4 -o slow.mp4
edv speed --factor 2.0 input.mp4 -o fast.mp4
```

### Filters and Effects

```bash
# Color adjustment
edv filter color --brightness 10 --contrast 1.2 --saturation 1.5 input.mp4 -o enhanced.mp4

# Blur effect
edv filter effect --type blur --strength 5 input.mp4 -o blurred.mp4

# Fade transition
edv filter transition --type fade --duration 2.5 video1.mp4 video2.mp4 -o transition.mp4
```

### Audio Processing

```bash
# Volume adjustment
edv audio volume --level 0.8 input.mp4 -o output.mp4
edv audio volume --db -3 input.mp4 -o output.mp4

# Audio extraction
edv audio extract input.mp4 -o audio.mp3
edv audio extract --format aac --bitrate 256k input.mp4 -o audio.aac

# Audio replacement
edv audio replace --audio audio.mp3 input.mp4 -o output.mp4
```

### Format Conversion

```bash
# Format conversion
edv convert --format mp4 input.mkv -o output.mp4
edv convert --codec h264 --preset slow --crf 18 input.mp4 -o high_quality.mp4

# Resolution change
edv convert resize --size 1280x720 input.mp4 -o output.mp4
edv convert resize --preset 480p input.mp4 -o output.mp4
```

### Batch Processing

```bash
# Convert all videos in a directory to MP4 format
edv batch convert --format mp4 --codec h264 ./input_dir/ -o ./output_dir/

# Save and apply templates
edv template save my_process.template
edv template apply my_process.template ./input_dir/ -o ./output_dir/
```

## Command List

- `trim`: Video trimming
- `cut`: Section removal
- `concat`: Video concatenation
- `filter`: Apply filters
- `audio`: Audio processing
- `convert`: Format conversion
- `subtitle`: Subtitle processing
- `timeline`: Timeline editing
- `batch`: Batch processing
- `project`: Project management

For detailed options, check the help command:

```bash
edv --help
edv <command> --help
```

## Technology Stack

- **Language**: Rust
- **Video Processing**: FFmpeg (Rust bindings like ffmpeg-next)
- **CLI Parsing**: clap
- **Parallel Processing**: rayon, tokio
- **Serialization**: serde
- **Error Handling**: anyhow, thiserror
- **Logging**: log, env_logger

## Supported Formats

### Input Formats

- MP4 (H.264, H.265, AAC, MP3)
- MOV (QuickTime, H.264, ProRes, AAC, PCM)
- MKV (Matroska, H.264, H.265, VP9, AAC, FLAC, Opus)
- AVI, WebM, TS, FLV, GIF, etc.

### Output Formats

- MP4 (H.264, H.265, AAC)
- MKV (Matroska, H.264, H.265, VP9, AAC, FLAC, Opus)
- WebM (VP8, VP9, Opus, Vorbis)
- GIF, MP3, WAV, OGG, etc.

## Operating Environments

- Linux (Ubuntu 20.04+, Debian 11+, CentOS/RHEL 8+)
- macOS (12+)
- Windows (10, 11) *WSL2 recommended

## Contributing

Bug reports, feature requests, and pull requests are welcome. For details, see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Released under the MIT License. See the [LICENSE](LICENSE) file for details. 