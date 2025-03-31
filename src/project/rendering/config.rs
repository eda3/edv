/// Configuration options for timeline rendering.
///
/// This module defines the configuration options for rendering a timeline
/// to a video file, including format selection, codec options, and quality settings.
use crate::utility::time::TimePosition;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Video codec types supported by the renderer.
///
/// This enum represents the different video codecs that can be used
/// for rendering video tracks. Each variant corresponds to a specific
/// codec implementation in FFmpeg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    /// Copy video stream without re-encoding
    Copy,
    /// H.264/AVC codec
    H264,
    /// H.265/HEVC codec
    H265,
    /// VP9 codec
    VP9,
    /// AV1 codec
    AV1,
}

impl Default for VideoCodec {
    fn default() -> Self {
        Self::H264
    }
}

impl VideoCodec {
    /// Converts the codec to its FFmpeg codec name.
    ///
    /// # Returns
    ///
    /// A string containing the FFmpeg codec name.
    pub fn to_ffmpeg_codec(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::H264 => "libx264",
            Self::H265 => "libx265",
            Self::VP9 => "libvpx-vp9",
            Self::AV1 => "libaom-av1",
        }
    }
}

/// Audio codec types supported by the renderer.
///
/// This enum represents the different audio codecs that can be used
/// for rendering audio tracks. Each variant corresponds to a specific
/// codec implementation in FFmpeg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioCodec {
    /// Copy audio stream without re-encoding
    Copy,
    /// AAC codec
    AAC,
    /// Opus codec
    Opus,
    /// MP3 codec
    MP3,
    /// Vorbis codec
    Vorbis,
}

impl Default for AudioCodec {
    fn default() -> Self {
        Self::AAC
    }
}

impl AudioCodec {
    /// Converts the codec to its FFmpeg codec name.
    ///
    /// # Returns
    ///
    /// A string containing the FFmpeg codec name.
    pub fn to_ffmpeg_codec(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::AAC => "aac",
            Self::Opus => "libopus",
            Self::MP3 => "libmp3lame",
            Self::Vorbis => "libvorbis",
        }
    }
}

/// Output format options for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// MP4 container format (default).
    MP4,
    /// WebM container format.
    WebM,
    /// MOV container format.
    MOV,
    /// MKV container format.
    MKV,
    /// GIF format (video only, no audio).
    GIF,
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
            Self::GIF => "gif",
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

    /// Whether to use cached assets when available.
    pub use_cache: bool,

    /// Whether to auto-load assets on project load.
    pub auto_load_assets: bool,

    /// Whether to optimize rendering for complex timelines.
    pub optimize_complex_timelines: bool,

    /// Cache directory (if None, uses the default cache directory).
    pub cache_dir: Option<PathBuf>,

    /// Maximum cache size in bytes (if None, no limit).
    pub max_cache_size: Option<u64>,
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
            use_cache: true,
            auto_load_assets: true,
            optimize_complex_timelines: true,
            cache_dir: None,
            max_cache_size: Some(10 * 1024 * 1024 * 1024), // 10 GB default
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

    /// Sets whether to use cached assets when available.
    #[must_use]
    pub fn with_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }

    /// Sets whether to auto-load assets on project load.
    #[must_use]
    pub fn with_auto_load_assets(mut self, auto_load: bool) -> Self {
        self.auto_load_assets = auto_load;
        self
    }

    /// Sets whether to optimize rendering for complex timelines.
    #[must_use]
    pub fn with_optimize_complex_timelines(mut self, optimize: bool) -> Self {
        self.optimize_complex_timelines = optimize;
        self
    }

    /// Sets the cache directory.
    #[must_use]
    pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = Some(dir);
        self
    }

    /// Sets the maximum cache size in bytes.
    #[must_use]
    pub fn with_max_cache_size(mut self, size: u64) -> Self {
        self.max_cache_size = Some(size);
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

        Ok(())
    }
}

impl PartialEq for RenderConfig {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && (self.frame_rate - other.frame_rate).abs() < 0.001
            && self.video_codec == other.video_codec
            && self.video_quality == other.video_quality
            && self.audio_codec == other.audio_codec
            && self.audio_quality == other.audio_quality
            && self.format == other.format
            && self.include_subtitles == other.include_subtitles
    }
}

impl Eq for RenderConfig {}

impl Hash for RenderConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.height.hash(state);

        // f64をハッシュ可能にするためにビット表現を使用
        let frame_rate_bits = self.frame_rate.to_bits();
        frame_rate_bits.hash(state);

        self.video_codec.hash(state);
        self.video_quality.hash(state);
        self.audio_codec.hash(state);
        self.audio_quality.hash(state);
        self.format.hash(state);
        self.include_subtitles.hash(state);
    }
}
