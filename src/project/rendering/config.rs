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

/// Hardware acceleration types supported by the renderer.
///
/// This enum represents different hardware acceleration methods that can
/// be used to accelerate video processing with hardware support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwareAccelType {
    /// No hardware acceleration, use CPU only
    None,
    /// NVIDIA GPU acceleration (NVENC/NVDEC)
    Nvidia,
    /// AMD GPU acceleration (AMF)
    Amd,
    /// Intel Quick Sync acceleration
    Intel,
    /// Video Acceleration API (for Linux)
    Vaapi,
    /// DirectX Video Acceleration (for Windows)
    Dxva2,
    /// Video Toolbox (for macOS)
    VideoToolbox,
    /// Auto-detect the best available hardware acceleration
    Auto,
}

impl Default for HardwareAccelType {
    fn default() -> Self {
        Self::None
    }
}

impl HardwareAccelType {
    /// Converts the hardware acceleration type to its FFmpeg hwaccel name.
    ///
    /// # Returns
    ///
    /// A string containing the FFmpeg hwaccel name, or None if no hardware acceleration.
    pub fn to_ffmpeg_hwaccel(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Nvidia => Some("cuda"),
            Self::Amd => Some("amf"),
            Self::Intel => Some("qsv"),
            Self::Vaapi => Some("vaapi"),
            Self::Dxva2 => Some("dxva2"),
            Self::VideoToolbox => Some("videotoolbox"),
            Self::Auto => None, // Auto will be handled separately in the compositor
        }
    }

    /// Gets the appropriate encoder name for this hardware acceleration type.
    ///
    /// # Arguments
    ///
    /// * `codec` - The base codec to get hardware accelerated version for
    ///
    /// # Returns
    ///
    /// A string containing the FFmpeg encoder name for hardware acceleration,
    /// or None if not applicable.
    pub fn get_hw_encoder_name(&self, codec: VideoCodec) -> Option<&'static str> {
        match (self, codec) {
            (Self::None, _) => None,
            (Self::Nvidia, VideoCodec::H264) => Some("h264_nvenc"),
            (Self::Nvidia, VideoCodec::H265) => Some("hevc_nvenc"),
            (Self::Amd, VideoCodec::H264) => Some("h264_amf"),
            (Self::Amd, VideoCodec::H265) => Some("hevc_amf"),
            (Self::Intel, VideoCodec::H264) => Some("h264_qsv"),
            (Self::Intel, VideoCodec::H265) => Some("hevc_qsv"),
            (Self::Vaapi, VideoCodec::H264) => Some("h264_vaapi"),
            (Self::Vaapi, VideoCodec::H265) => Some("hevc_vaapi"),
            (Self::VideoToolbox, VideoCodec::H264) => Some("h264_videotoolbox"),
            (Self::VideoToolbox, VideoCodec::H265) => Some("hevc_videotoolbox"),
            // Either Auto or no hardware support for this combination
            _ => None,
        }
    }

    /// Detects available hardware acceleration methods on the current system.
    ///
    /// # Returns
    ///
    /// A vector of available hardware acceleration types.
    pub fn detect_available() -> Vec<Self> {
        let mut available = vec![Self::None]; // None is always available

        // Check for NVIDIA GPUs (simplified - in real impl would check for CUDA-capable devices)
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        if std::path::Path::new("/dev/nvidia0").exists()
            || std::env::var("CUDA_VISIBLE_DEVICES").is_ok()
        {
            available.push(Self::Nvidia);
        }

        // Check for AMD GPUs (simplified)
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        if std::path::Path::new("/dev/dri/renderD128").exists() {
            available.push(Self::Amd);
            available.push(Self::Vaapi); // VAAPI likely available on Linux with AMD
        }

        // Intel QuickSync check (simplified)
        if std::env::var("INTEL_GPU").is_ok() || std::env::var("LIBVA_DRIVER_NAME").is_ok() {
            available.push(Self::Intel);
        }

        // Platform specific checks
        #[cfg(target_os = "windows")]
        available.push(Self::Dxva2);

        #[cfg(target_os = "macos")]
        available.push(Self::VideoToolbox);

        // If we found any hardware acceleration method, add Auto option
        if available.len() > 1 {
            available.push(Self::Auto);
        }

        available
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

    /// Hardware acceleration type to use.
    pub hardware_accel_type: HardwareAccelType,

    /// Whether to use hardware accelerated decoding when available.
    pub use_hw_decoding: bool,

    /// Maximum GPU memory usage in bytes (if None, no limit).
    pub max_gpu_memory: Option<u64>,
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
            hardware_accel_type: HardwareAccelType::default(),
            use_hw_decoding: true,
            max_gpu_memory: None,
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

    /// Sets the hardware acceleration type to use.
    #[must_use]
    pub fn with_hardware_acceleration(mut self, accel_type: HardwareAccelType) -> Self {
        self.hardware_accel_type = accel_type;
        self
    }

    /// Sets whether to use hardware accelerated decoding.
    #[must_use]
    pub fn with_hw_decoding(mut self, use_hw_decoding: bool) -> Self {
        self.use_hw_decoding = use_hw_decoding;
        self
    }

    /// Sets the maximum GPU memory usage in bytes.
    #[must_use]
    pub fn with_max_gpu_memory(mut self, size: u64) -> Self {
        self.max_gpu_memory = Some(size);
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

        // Validate hardware acceleration type for the selected codec
        if self.hardware_accel_type != HardwareAccelType::None
            && self.hardware_accel_type != HardwareAccelType::Auto
        {
            // Check if the selected codec is compatible with the hardware acceleration
            if self
                .hardware_accel_type
                .get_hw_encoder_name(self.video_codec)
                .is_none()
            {
                return Err(format!(
                    "Selected video codec ({:?}) is not compatible with hardware acceleration type ({:?})",
                    self.video_codec, self.hardware_accel_type
                ));
            }
        }

        Ok(())
    }

    /// Determines whether hardware acceleration should be used based on current config.
    ///
    /// # Returns
    ///
    /// True if hardware acceleration should be used, false otherwise.
    pub fn should_use_hardware_acceleration(&self) -> bool {
        // Don't use hardware acceleration if explicitly set to None
        if self.hardware_accel_type == HardwareAccelType::None {
            return false;
        }

        // Auto means we need to check if hardware acceleration is available
        if self.hardware_accel_type == HardwareAccelType::Auto {
            // In a real implementation, we would check available hardware here
            // For now, we'll say none is available
            return false;
        }

        // Check if selected codec is supported with this hardware acceleration
        self.hardware_accel_type
            .get_hw_encoder_name(self.video_codec)
            .is_some()
    }
}

impl PartialEq for RenderConfig {
    fn eq(&self, other: &Self) -> bool {
        self.output_path == other.output_path
            && self.width == other.width
            && self.height == other.height
            && self.frame_rate == other.frame_rate
            && self.video_codec == other.video_codec
            && self.video_quality == other.video_quality
            && self.audio_codec == other.audio_codec
            && self.audio_quality == other.audio_quality
            && self.format == other.format
            && self.start_position == other.start_position
            && self.end_position == other.end_position
            && self.threads == other.threads
            && self.include_subtitles == other.include_subtitles
            && self.hardware_accel_type == other.hardware_accel_type
            && self.use_hw_decoding == other.use_hw_decoding
    }
}

impl Eq for RenderConfig {}

impl Hash for RenderConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.output_path.hash(state);
        self.width.hash(state);
        self.height.hash(state);
        // We'll ignore frame_rate in the hash since it's an f64
        self.video_codec.hash(state);
        self.video_quality.hash(state);
        self.audio_codec.hash(state);
        self.audio_quality.hash(state);
        self.format.hash(state);
        // We'll ignore start_position and end_position in the hash
        // since they're optional TimePositions
        self.threads.hash(state);
        self.include_subtitles.hash(state);
        self.hardware_accel_type.hash(state);
        self.use_hw_decoding.hash(state);
    }
}
