# edv - Audio Module Implementation

This document provides detailed implementation guidelines for the Audio module of the edv application.

## Overview

The Audio module provides comprehensive functionality for working with audio in video files, including volume adjustment, audio extraction, audio replacement, audio fading, and other audio processing capabilities. It serves as the foundation for all audio-related operations within the edv application, enabling users to manipulate audio tracks with precision and flexibility.

## Structure

```
src/audio/
├── mod.rs      // Module exports and common definitions
├── common.rs   // Shared utilities and constants
├── error.rs    // Error types and handling
├── volume.rs   // Volume adjustment functionality
├── extractor.rs // Audio extraction functionality 
├── replacer.rs  // Audio replacement functionality
└── fade.rs      // Audio fading effects
```

## Key Components

### Core Module Structure (mod.rs)

The main module file exports the public API and defines common functionality:

```rust
pub use self::error::{Error, Result};

pub mod error;
pub mod extractor;
pub mod fade;
pub mod replacer;
pub mod volume;

/// Common audio processing constants and utilities.
pub mod common {
    /// Default audio bitrate used for encoding when not specified (192 kbps).
    pub const DEFAULT_AUDIO_BITRATE: &str = "192k";

    /// Default audio sample rate (44.1 kHz).
    pub const DEFAULT_SAMPLE_RATE: u32 = 44100;

    /// Default audio codec used for encoding when not specified.
    pub const DEFAULT_AUDIO_CODEC: &str = "aac";

    /// Standard audio file formats supported for extraction.
    pub const SUPPORTED_AUDIO_FORMATS: &[&str] = &["mp3", "aac", "wav", "flac", "ogg"];
    
    /// Converts dB value to a linear multiplier.
    #[must_use]
    pub fn db_to_linear(db: f64) -> f64 {
        10.0_f64.powf(db / 20.0)
    }

    /// Converts linear multiplier to dB value.
    #[must_use]
    pub fn linear_to_db(linear: f64) -> f64 {
        20.0 * linear.max(1e-10).log10()
    }
}
```

### Error Handling (error.rs)

The error component defines a comprehensive error handling system:

```rust
/// Result type for audio operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during audio processing operations.
#[derive(Error, Debug)]
pub enum Error {
    /// The specified audio format is not supported.
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    /// The audio stream was not found in the input file.
    #[error("No audio stream found in {0}")]
    NoAudioStream(PathBuf),

    /// The audio file could not be processed due to invalid data.
    #[error("Invalid audio data in {file}: {reason}")]
    InvalidAudioData {
        /// Path to the file with invalid data.
        file: PathBuf,
        /// Reason why the data is invalid.
        reason: String,
    },
    
    /// Invalid volume level specified.
    #[error("Invalid volume level: {0}")]
    InvalidVolumeLevel(f64),

    /// Invalid fade duration specified.
    #[error("Invalid fade duration: {0}")]
    InvalidFadeDuration(f64),
    
    // Additional error variants...
}

impl Error {
    /// Creates a new `UnsupportedFormat` error.
    #[must_use]
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat(format.into())
    }
    
    // Additional factory methods...
}
```

### Volume Adjustment (volume.rs)

The volume component provides functionality for adjusting audio volume:

```rust
/// Adjustment method for volume operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeAdjustment {
    /// Linear multiplier (e.g., 1.5 = 150% volume)
    Linear(f64),

    /// Decibel adjustment (e.g., 6.0 = +6dB)
    Decibel(f64),
}

impl VolumeAdjustment {
    /// Converts the adjustment to a linear multiplier.
    #[must_use]
    pub fn as_linear(&self) -> f64 {
        match *self {
            Self::Linear(value) => value,
            Self::Decibel(db) => common::db_to_linear(db),
        }
    }

    /// Converts the adjustment to decibels.
    #[must_use]
    pub fn as_db(&self) -> f64 {
        match *self {
            Self::Linear(value) => common::linear_to_db(value),
            Self::Decibel(db) => db,
        }
    }
}

/// Parameters for a temporal volume adjustment.
#[derive(Debug, Clone)]
pub struct TemporalAdjustment {
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds (None means until the end)
    pub end_time: Option<f64>,
    /// Volume adjustment to apply
    pub adjustment: VolumeAdjustment,
}

/// Adjusts the volume of all audio tracks in a video file.
pub fn adjust_volume<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    adjustment: VolumeAdjustment,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}

/// Adjusts the volume of audio within specific time ranges.
pub fn adjust_volume_temporal<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    adjustments: &[TemporalAdjustment],
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}

/// Normalizes the audio volume to the target level.
pub fn normalize_volume<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    target_level: f64,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}
```

### Audio Extraction (extractor.rs)

The extractor component handles extracting audio from video files:

```rust
/// Audio extraction format options.
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// The audio codec to use for extraction (e.g., "aac", "mp3", "flac")
    pub codec: String,

    /// The bitrate for the extracted audio (e.g., "192k")
    pub bitrate: String,

    /// The sample rate for the extracted audio (e.g., 44100)
    pub sample_rate: u32,

    /// Number of audio channels (1=mono, 2=stereo)
    pub channels: u8,

    /// Which audio stream to extract (None = all streams)
    pub stream_index: Option<usize>,

    /// Normalize audio during extraction
    pub normalize_audio: bool,

    /// Start time to extract from (in seconds)
    pub start_time: Option<f64>,

    /// Duration to extract (in seconds)
    pub duration: Option<f64>,
}

impl ExtractionOptions {
    /// Creates a new instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    
    // Builder methods for configuring options...
}

/// Extracts audio from a video file using the specified options.
pub fn extract_audio<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    options: &ExtractionOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}

/// Lists all audio streams in a video file.
pub fn list_audio_streams<P>(
    ffmpeg: &FFmpeg, 
    input: P
) -> Result<Vec<(usize, String, u8, u32)>>
where
    P: AsRef<Path>,
{
    // Implementation details...
}

/// Extracts multiple segments from a video file's audio.
pub fn extract_segments<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    segments: &[(f64, f64)],
    options: &ExtractionOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}
```

### Audio Replacement (replacer.rs)

The replacer component provides functionality for replacing audio in video files:

```rust
/// Options for audio replacement operations.
#[derive(Debug, Clone)]
pub struct ReplacementOptions {
    /// Whether to preserve original video codec
    pub preserve_video_codec: bool,
    
    /// Output video codec (when not preserving)
    pub video_codec: String,
    
    /// Whether to normalize audio volume
    pub normalize_audio: bool,
    
    /// Audio codec to use
    pub audio_codec: String,
    
    /// Audio bitrate to use
    pub audio_bitrate: String,
    
    /// Whether to mix with original audio (true) or replace entirely (false)
    pub mix_with_original: bool,
    
    /// Volume level for new audio (0.0-1.0) when mixing
    pub new_audio_volume: f64,
    
    /// Volume level for original audio (0.0-1.0) when mixing
    pub original_audio_volume: f64,
    
    /// Offset in seconds to apply to the new audio (positive = delay)
    pub audio_offset: f64,
}

/// Replaces the audio track in a video file.
pub fn replace_audio<P1, P2, P3>(
    ffmpeg: &FFmpeg,
    video_input: P1,
    audio_input: P2,
    output: P3,
    options: &ReplacementOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    // Implementation details...
}

/// Replaces audio in a specific time range of a video.
pub fn replace_audio_range<P1, P2, P3>(
    ffmpeg: &FFmpeg,
    video_input: P1,
    audio_input: P2,
    output: P3,
    start_time: f64,
    end_time: Option<f64>,
    options: &ReplacementOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    // Implementation details...
}

/// Mutes a specific section of a video file.
pub fn mute_section<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    start_time: f64,
    end_time: f64,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}
```

### Audio Fading (fade.rs)

The fade component provides functionality for creating audio fades:

```rust
/// Types of fade curves available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurve {
    /// Linear fade (constant rate of change)
    Linear,
    
    /// Exponential fade (starts slow, ends fast)
    Exponential,
    
    /// Logarithmic fade (starts fast, ends slow)
    Logarithmic,
    
    /// Sinusoidal fade (smooth S-curve)
    Sinusoidal,
}

/// Applies a fade-in effect to audio.
pub fn fade_in<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    duration: f64,
    curve: FadeCurve,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}

/// Applies a fade-out effect to audio.
pub fn fade_out<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    duration: f64,
    curve: FadeCurve,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}

/// Applies both fade-in and fade-out to audio.
pub fn fade_in_out<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    fade_in_duration: f64,
    fade_out_duration: f64,
    curve: FadeCurve,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Implementation details...
}

/// Applies a crossfade between two audio files.
pub fn crossfade<P1, P2, P3>(
    ffmpeg: &FFmpeg,
    input1: P1,
    input2: P2,
    output: P3,
    crossfade_duration: f64,
    curve: FadeCurve,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    // Implementation details...
}
```

## Implementation Status

The Audio module has been implemented with the following features:

1. **Volume Adjustment**:
   - Global volume adjustment (linear and decibel scaling) ✅
   - Temporal volume adjustment (different volumes at different times) ✅
   - Audio normalization ✅

2. **Audio Extraction**:
   - Full audio track extraction ✅
   - Time-range extraction ✅
   - Multi-segment extraction ✅
   - Format and codec selection ✅
   - Channel and bitrate configuration ✅

3. **Audio Replacement**:
   - Full track replacement ✅
   - Mixing new audio with original ✅
   - Time-range replacement ✅
   - Audio offset adjustment ✅
   - Section muting ✅

4. **Audio Fading**:
   - Fade-in effects ✅
   - Fade-out effects ✅
   - Combined fade-in/fade-out ✅
   - Multiple curve types (linear, exponential, logarithmic, sinusoidal) ✅
   - Cross-fading between audio tracks ✅

5. **Integration**:
   - FFmpeg integration for processing ✅
   - Error handling and reporting ✅
   - Common utilities and constants ✅

## Current Limitations

1. **Advanced Audio Processing**:
   - Limited support for audio filters beyond basic operations
   - No direct equalization or compression support
   - No support for multi-track mixing (beyond simple 2-track operations)

2. **Performance Optimization**:
   - Large audio files may experience performance issues
   - Memory usage optimizations needed for very large files

3. **Format Support**:
   - Limited to the most common audio formats
   - Some specialized audio codecs may not be supported

## Future Development

The following enhancements are planned for the Audio module:

1. **Enhanced Audio Processing**:
   - Advanced equalization capabilities
   - Audio compression and limiting
   - Noise reduction and audio restoration
   - Audio spectrum analysis

2. **Multi-track Operations**:
   - Support for mixing multiple audio tracks
   - Advanced channel mapping and routing
   - Surround sound support (5.1, 7.1)
   - Audio synchronization tools

3. **Performance Improvements**:
   - Optimized processing for large files
   - Parallel processing for audio operations
   - Streaming processing for reduced memory usage

4. **Integration Enhancements**:
   - Improved timeline integration with waveform visualization
   - Real-time preview capabilities
   - Enhanced metadata extraction and modification

## Usage Examples

### Basic Volume Adjustment

```rust
use edv::ffmpeg::FFmpeg;
use edv::audio::volume::{adjust_volume, VolumeAdjustment};
use std::path::Path;

// Initialize FFmpeg
let ffmpeg = FFmpeg::detect().expect("Failed to find FFmpeg");

// Adjust volume by 6dB
let adjustment = VolumeAdjustment::Decibel(6.0);
adjust_volume(
    &ffmpeg,
    Path::new("input.mp4"),
    Path::new("output.mp4"),
    adjustment
).expect("Failed to adjust volume");
```

### Extracting Audio from a Video

```rust
use edv::ffmpeg::FFmpeg;
use edv::audio::extractor::{extract_audio, ExtractionOptions};
use std::path::Path;

// Initialize FFmpeg
let ffmpeg = FFmpeg::detect().expect("Failed to find FFmpeg");

// Configure extraction options
let options = ExtractionOptions::new()
    .codec("mp3")
    .bitrate("320k")
    .sample_rate(48000)
    .channels(2)
    .start_time(30.0)
    .duration(60.0);

// Extract audio
extract_audio(
    &ffmpeg,
    Path::new("video.mp4"),
    Path::new("audio.mp3"),
    &options
).expect("Failed to extract audio");
```

### Replacing Audio in a Video

```rust
use edv::ffmpeg::FFmpeg;
use edv::audio::replacer::{replace_audio, ReplacementOptions};
use std::path::Path;

// Initialize FFmpeg
let ffmpeg = FFmpeg::detect().expect("Failed to find FFmpeg");

// Configure replacement options
let options = ReplacementOptions::new()
    .preserve_video_codec(true)
    .mix_with_original(true)
    .new_audio_volume(0.8)
    .original_audio_volume(0.2);

// Replace audio
replace_audio(
    &ffmpeg,
    Path::new("video.mp4"),
    Path::new("new_audio.mp3"),
    Path::new("output.mp4"),
    &options
).expect("Failed to replace audio");
```

### Creating Audio Fades

```rust
use edv::ffmpeg::FFmpeg;
use edv::audio::fade::{fade_in_out, FadeCurve};
use std::path::Path;

// Initialize FFmpeg
let ffmpeg = FFmpeg::detect().expect("Failed to find FFmpeg");

// Apply fade-in (3 seconds) and fade-out (5 seconds)
fade_in_out(
    &ffmpeg,
    Path::new("input.mp4"),
    Path::new("output.mp4"),
    3.0,  // fade-in duration
    5.0,  // fade-out duration
    FadeCurve::Sinusoidal
).expect("Failed to apply fades");
``` 