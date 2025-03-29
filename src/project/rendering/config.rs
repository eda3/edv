/// Configuration options for timeline rendering.
///
/// This module defines the configuration options for rendering a timeline
/// to a video file, including format selection, codec options, and quality settings.
use crate::utility::time::TimePosition;
use std::path::PathBuf;

/// Video codec options for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    /// H.264 codec (default).
    H264,
    /// H.265/HEVC codec.
    H265,
    /// VP9 codec.
    VP9,
    /// ProRes codec.
    ProRes,
    /// Use stream copying when possible (no re-encoding).
    Copy,
}

impl Default for VideoCodec {
    fn default() -> Self {
        Self::H264
    }
}

impl VideoCodec {
    /// Gets the FFmpeg codec name for this codec.
    #[must_use]
    pub fn to_ffmpeg_codec(&self) -> &'static str {
        match self {
            Self::H264 => "libx264",
            Self::H265 => "libx265",
            Self::VP9 => "libvpx-vp9",
            Self::ProRes => "prores",
            Self::Copy => "copy",
        }
    }
}

/// Audio codec options for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    /// AAC codec (default).
    AAC,
    /// MP3 codec.
    MP3,
    /// Opus codec.
    Opus,
    /// FLAC codec.
    FLAC,
    /// Use stream copying when possible (no re-encoding).
    Copy,
}

impl Default for AudioCodec {
    fn default() -> Self {
        Self::AAC
    }
}

impl AudioCodec {
    /// Gets the FFmpeg codec name for this codec.
    #[must_use]
    pub fn to_ffmpeg_codec(&self) -> &'static str {
        match self {
            Self::AAC => "aac",
            Self::MP3 => "libmp3lame",
            Self::Opus => "libopus",
            Self::FLAC => "flac",
            Self::Copy => "copy",
        }
    }
}

/// Output format options for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// MP4 container format (default).
    MP4,
    /// WebM container format.
    WebM,
    /// MOV container format.
    MOV,
    /// MKV container format.
    MKV,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::MP4
    }
}

impl OutputFormat {
    /// Gets the file extension for this format.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::MP4 => "mp4",
            Self::WebM => "webm",
            Self::MOV => "mov",
            Self::MKV => "mkv",
        }
    }
}

/// Configuration for timeline rendering.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Output file path.
    pub output_path: PathBuf,

    /// Video resolution width (in pixels).
    pub width: u32,

    /// Video resolution height (in pixels).
    pub height: u32,

    /// Video frame rate (frames per second).
    pub frame_rate: f64,

    /// Video codec to use.
    pub video_codec: VideoCodec,

    /// Video quality (1-100, higher is better).
    pub video_quality: u32,

    /// Audio codec to use.
    pub audio_codec: AudioCodec,

    /// Audio quality (1-100, higher is better).
    pub audio_quality: u32,

    /// Output container format.
    pub format: OutputFormat,

    /// Start position for rendering (default: beginning of timeline).
    pub start_position: Option<TimePosition>,

    /// End position for rendering (default: end of timeline).
    pub end_position: Option<TimePosition>,

    /// Number of render threads to use (default: available CPU cores).
    pub threads: Option<usize>,

    /// Whether to include subtitles in the output.
    pub include_subtitles: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            output_path: PathBuf::new(),
            width: 1920,
            height: 1080,
            frame_rate: 30.0,
            video_codec: VideoCodec::default(),
            video_quality: 80,
            audio_codec: AudioCodec::default(),
            audio_quality: 80,
            format: OutputFormat::default(),
            start_position: None,
            end_position: None,
            threads: None,
            include_subtitles: true,
        }
    }
}

impl RenderConfig {
    /// Creates a new render configuration with default settings.
    #[must_use]
    pub fn new(output_path: PathBuf) -> Self {
        Self {
            output_path,
            ..Default::default()
        }
    }

    /// Sets the video resolution.
    #[must_use]
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the video frame rate.
    #[must_use]
    pub fn with_frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    /// Sets the video codec and quality.
    #[must_use]
    pub fn with_video_settings(mut self, codec: VideoCodec, quality: u32) -> Self {
        self.video_codec = codec;
        self.video_quality = quality.min(100);
        self
    }

    /// Sets the audio codec and quality.
    #[must_use]
    pub fn with_audio_settings(mut self, codec: AudioCodec, quality: u32) -> Self {
        self.audio_codec = codec;
        self.audio_quality = quality.min(100);
        self
    }

    /// Sets the output container format.
    #[must_use]
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets the rendering range.
    #[must_use]
    pub fn with_range(mut self, start: Option<TimePosition>, end: Option<TimePosition>) -> Self {
        self.start_position = start;
        self.end_position = end;
        self
    }

    /// Sets the number of render threads.
    #[must_use]
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// Sets whether to include subtitles in the output.
    #[must_use]
    pub fn with_subtitles(mut self, include: bool) -> Self {
        self.include_subtitles = include;
        self
    }

    /// Validates the configuration and returns an error if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.output_path.as_os_str().is_empty() {
            return Err("Output path cannot be empty".to_string());
        }

        if self.width == 0 || self.height == 0 {
            return Err("Video dimensions cannot be zero".to_string());
        }

        if self.frame_rate <= 0.0 {
            return Err("Frame rate must be positive".to_string());
        }

        // Add more validation as needed

        Ok(())
    }
}
