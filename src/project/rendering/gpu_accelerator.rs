/// GPU acceleration for rendering.
///
/// This module provides functionality for GPU-accelerated rendering operations,
/// optimizing performance by utilizing available hardware acceleration.
use crate::ffmpeg::{Error as FFmpegError, FFmpeg, Result as FFmpegResult};
use crate::project::rendering::config::{HardwareAccelType, RenderConfig, VideoCodec};
use crate::utility::gpu_test::{self, GpuTestResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Result of GPU performance benchmarking.
#[derive(Debug, Clone)]
pub struct GpuBenchmarkResult {
    /// Hardware acceleration type tested.
    pub accel_type: HardwareAccelType,

    /// Whether this acceleration method is available.
    pub is_available: bool,

    /// Performance score (higher is better).
    pub performance_score: Option<u32>,

    /// Time taken for test encoding (milliseconds).
    pub encoding_time_ms: Option<u64>,

    /// Time taken for test decoding (milliseconds).
    pub decoding_time_ms: Option<u64>,
}

/// Represents a GPU accelerator capable of hardware-accelerated encoding and decoding.
#[derive(Debug, Clone)]
pub struct GpuAccelerator {
    /// FFmpeg instance.
    ffmpeg: Arc<FFmpeg>,

    /// Available hardware acceleration types.
    available_types: Vec<HardwareAccelType>,

    /// Performance test results for each acceleration type.
    benchmark_results: HashMap<HardwareAccelType, GpuBenchmarkResult>,

    /// The currently selected acceleration type.
    selected_type: HardwareAccelType,

    /// Whether hardware decoding should be used.
    use_hw_decoding: bool,

    /// Maximum GPU memory to use (bytes).
    max_gpu_memory: Option<u64>,

    /// Whether GPU acceleration is enabled.
    enabled: bool,
}

impl GpuAccelerator {
    /// Creates a new GPU accelerator instance.
    ///
    /// # Arguments
    ///
    /// * `ffmpeg` - Reference to the FFmpeg instance
    ///
    /// # Returns
    ///
    /// A new `GpuAccelerator` instance.
    pub fn new(ffmpeg: Arc<FFmpeg>) -> Self {
        let available_types = HardwareAccelType::detect_available();
        let selected_type = HardwareAccelType::None;

        Self {
            ffmpeg,
            available_types,
            benchmark_results: HashMap::new(),
            selected_type,
            use_hw_decoding: false,
            max_gpu_memory: None,
            enabled: false,
        }
    }

    /// Initializes the GPU accelerator with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The render configuration to use
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if initialization failed.
    pub fn initialize(&mut self, config: &RenderConfig) -> FFmpegResult<()> {
        // Set configuration options
        self.use_hw_decoding = config.use_hw_decoding;
        self.max_gpu_memory = config.max_gpu_memory;

        // Decide whether to enable GPU acceleration
        self.enabled = config.should_use_hardware_acceleration();

        // If configured to auto-select hardware acceleration or a specific type is requested
        if self.enabled {
            if config.hardware_accel_type == HardwareAccelType::Auto {
                // Auto-select the best hardware acceleration type
                self.autoselect_best_acceleration()?;
            } else {
                // Use the specific hardware acceleration type if available
                self.select_acceleration_type(config.hardware_accel_type)?;
            }
        } else {
            // Explicitly disabled - use CPU only
            self.selected_type = HardwareAccelType::None;
        }

        Ok(())
    }

    /// Auto-selects the best hardware acceleration type based on benchmarks.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if selecting failed.
    pub fn autoselect_best_acceleration(&mut self) -> FFmpegResult<()> {
        // Run benchmarks if not already done
        if self.benchmark_results.is_empty() {
            self.run_benchmarks()?;
        }

        // Find the best performing GPU
        self.selected_type = gpu_test::select_best_hardware_acceleration(&self.ffmpeg);

        Ok(())
    }

    /// Selects a specific hardware acceleration type.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type to select
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if the type is not available.
    pub fn select_acceleration_type(&mut self, accel_type: HardwareAccelType) -> FFmpegResult<()> {
        // Skip if it's None (CPU)
        if accel_type == HardwareAccelType::None {
            self.selected_type = HardwareAccelType::None;
            self.enabled = false;
            return Ok(());
        }

        // Check if the requested type is available
        if !self.available_types.contains(&accel_type) {
            return Err(FFmpegError::InvalidArgument(format!(
                "Hardware acceleration type {:?} is not available on this system",
                accel_type
            )));
        }

        // Set the selected type
        self.selected_type = accel_type;
        self.enabled = true;

        Ok(())
    }

    /// Runs benchmarks on all available hardware acceleration types.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if benchmarking failed.
    pub fn run_benchmarks(&mut self) -> FFmpegResult<()> {
        // Clear previous results
        self.benchmark_results.clear();

        // Create a test video
        let test_video = std::env::temp_dir().join("edv_gpu_test_input.mp4");
        gpu_test::create_test_video(&self.ffmpeg, &test_video, 5)?;

        // Test each acceleration type
        for &accel_type in &self.available_types {
            // Skip CPU-only
            if accel_type == HardwareAccelType::None || accel_type == HardwareAccelType::Auto {
                continue;
            }

            let mut result = GpuBenchmarkResult {
                accel_type,
                is_available: false,
                performance_score: None,
                encoding_time_ms: None,
                decoding_time_ms: None,
            };

            // Try to encode with this acceleration
            match gpu_test::test_gpu_encoding(&self.ffmpeg, accel_type, &test_video) {
                Ok(encoding_time) => {
                    // Successful test
                    result.is_available = true;
                    result.encoding_time_ms = Some(encoding_time);

                    // Calculate a simple performance score (inverse of encoding time)
                    // Higher score means better performance
                    if encoding_time > 0 {
                        result.performance_score = Some((10000 / encoding_time).min(100) as u32);
                    }
                }
                Err(_) => {
                    // Failed test - acceleration not available
                    result.is_available = false;
                }
            }

            // Store the result
            self.benchmark_results.insert(accel_type, result);
        }

        // Clean up test video
        let _ = std::fs::remove_file(test_video);

        Ok(())
    }

    /// Gets whether GPU acceleration is enabled.
    ///
    /// # Returns
    ///
    /// `true` if GPU acceleration is enabled, `false` otherwise.
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.selected_type != HardwareAccelType::None
    }

    /// Gets the currently selected hardware acceleration type.
    ///
    /// # Returns
    ///
    /// The currently selected hardware acceleration type.
    pub fn get_acceleration_type(&self) -> HardwareAccelType {
        self.selected_type
    }

    /// Gets benchmark results for all tested acceleration types.
    ///
    /// # Returns
    ///
    /// A map of benchmark results for each tested acceleration type.
    pub fn get_benchmark_results(&self) -> &HashMap<HardwareAccelType, GpuBenchmarkResult> {
        &self.benchmark_results
    }

    /// Gets the appropriate FFmpeg codec name for the current acceleration.
    ///
    /// # Arguments
    ///
    /// * `codec` - The video codec to get the accelerated version for
    ///
    /// # Returns
    ///
    /// The name of the FFmpeg codec to use, or the default CPU codec if
    /// acceleration is not available.
    pub fn get_encoder_name(&self, codec: VideoCodec) -> &'static str {
        if self.is_enabled() {
            // Try to get the hardware encoder name
            if let Some(encoder) = self.selected_type.get_hw_encoder_name(codec) {
                return encoder;
            }
        }

        // Fall back to software encoder
        codec.to_ffmpeg_codec()
    }

    /// Gets appropriate FFmpeg decoder options for hardware-accelerated decoding.
    ///
    /// # Returns
    ///
    /// A vector of FFmpeg arguments for hardware-accelerated decoding,
    /// or an empty vector if hardware decoding is not enabled.
    pub fn get_decoder_options(&self) -> Vec<String> {
        if !self.is_enabled() || !self.use_hw_decoding {
            return Vec::new();
        }

        let mut options = Vec::new();

        // Add hardware decoding options based on selected acceleration type
        if let Some(hwaccel) = self.selected_type.to_ffmpeg_hwaccel() {
            options.push("-hwaccel".to_string());
            options.push(hwaccel.to_string());

            // Add type-specific options
            match self.selected_type {
                HardwareAccelType::Nvidia => {
                    options.push("-hwaccel_output_format".to_string());
                    options.push("cuda".to_string());
                }
                HardwareAccelType::Vaapi => {
                    options.push("-hwaccel_output_format".to_string());
                    options.push("vaapi".to_string());
                }
                HardwareAccelType::VideoToolbox => {
                    options.push("-hwaccel_output_format".to_string());
                    options.push("videotoolbox".to_string());
                }
                _ => {}
            }
        }

        options
    }

    /// Gets appropriate FFmpeg encoder options for hardware-accelerated encoding.
    ///
    /// # Arguments
    ///
    /// * `codec` - The video codec to get options for
    /// * `quality` - Video quality (1-100)
    ///
    /// # Returns
    ///
    /// A vector of FFmpeg arguments for hardware-accelerated encoding.
    pub fn get_encoder_options(&self, codec: VideoCodec, quality: u32) -> Vec<String> {
        let mut options = Vec::new();

        if !self.is_enabled() {
            // Software encoding options based on codec and quality
            match codec {
                VideoCodec::H264 => {
                    options.push("-preset".to_string());

                    // Map quality to preset (slower = better quality)
                    let preset = if quality > 90 {
                        "veryslow"
                    } else if quality > 75 {
                        "slower"
                    } else if quality > 60 {
                        "slow"
                    } else if quality > 40 {
                        "medium"
                    } else if quality > 20 {
                        "fast"
                    } else {
                        "veryfast"
                    };

                    options.push(preset.to_string());
                    options.push("-crf".to_string());

                    // Reverse the quality to crf (lower crf = higher quality)
                    let crf = 30 - ((quality as f32 / 100.0) * 28.0).round() as u32;
                    options.push(crf.to_string());
                }
                VideoCodec::H265 => {
                    options.push("-preset".to_string());

                    // Map quality to preset
                    let preset = if quality > 90 {
                        "veryslow"
                    } else if quality > 75 {
                        "slower"
                    } else if quality > 60 {
                        "slow"
                    } else if quality > 40 {
                        "medium"
                    } else if quality > 20 {
                        "fast"
                    } else {
                        "veryfast"
                    };

                    options.push(preset.to_string());
                    options.push("-crf".to_string());

                    // HEVC typically uses slightly higher CRF values
                    let crf = 33 - ((quality as f32 / 100.0) * 30.0).round() as u32;
                    options.push(crf.to_string());
                }
                VideoCodec::VP9 => {
                    // VP9 has different quality settings
                    options.push("-b:v".to_string());
                    let bitrate = ((quality as f32 / 100.0) * 8000.0 + 1000.0).round() as u32;
                    options.push(format!("{}k", bitrate));
                }
                VideoCodec::AV1 => {
                    options.push("-crf".to_string());
                    let crf = 35 - ((quality as f32 / 100.0) * 30.0).round() as u32;
                    options.push(crf.to_string());
                    options.push("-cpu-used".to_string());

                    // CPU usage: 0 (slow, best quality) to 8 (fast, lowest quality)
                    let cpu_used = 8 - ((quality as f32 / 100.0) * 8.0).round() as u32;
                    options.push(cpu_used.to_string());
                }
                VideoCodec::Copy => {
                    // No encoding options for copy
                }
            }
        } else {
            // Hardware encoding options
            match self.selected_type {
                HardwareAccelType::Nvidia => {
                    // NVIDIA-specific options
                    options.push("-preset".to_string());

                    // Map quality to preset
                    let preset = if quality > 80 {
                        "slow" // P7
                    } else if quality > 60 {
                        "medium" // P4
                    } else if quality > 40 {
                        "fast" // P2
                    } else {
                        "fastest" // P1
                    };

                    options.push(preset.to_string());

                    // Rate control mode
                    options.push("-rc".to_string());
                    options.push("vbr".to_string());

                    // Target quality
                    let qp = 30 - ((quality as f32 / 100.0) * 25.0).round() as u32;
                    options.push("-qp".to_string());
                    options.push(qp.to_string());

                    // Bitrate
                    options.push("-b:v".to_string());
                    let bitrate = ((quality as f32 / 100.0) * 6000.0 + 2000.0).round() as u32;
                    options.push(format!("{}k", bitrate));
                }
                HardwareAccelType::Amd => {
                    // AMD-specific options
                    options.push("-quality".to_string());

                    let quality_level = if quality > 80 {
                        "quality"
                    } else if quality > 50 {
                        "balanced"
                    } else {
                        "speed"
                    };

                    options.push(quality_level.to_string());

                    // Control bitrate
                    options.push("-b:v".to_string());
                    let bitrate = ((quality as f32 / 100.0) * 8000.0 + 1000.0).round() as u32;
                    options.push(format!("{}k", bitrate));
                }
                HardwareAccelType::Intel => {
                    // Intel QuickSync options
                    options.push("-global_quality".to_string());
                    let global_quality = ((quality as f32 / 100.0) * 30.0).round() as u32;
                    options.push(global_quality.to_string());

                    options.push("-preset".to_string());

                    let preset = if quality > 80 {
                        "veryslow"
                    } else if quality > 60 {
                        "slower"
                    } else if quality > 40 {
                        "medium"
                    } else {
                        "fast"
                    };

                    options.push(preset.to_string());
                }
                HardwareAccelType::Vaapi => {
                    // VAAPI-specific options
                    options.push("-global_quality".to_string());
                    let global_quality = ((quality as f32 / 100.0) * 80.0).round() as u32;
                    options.push(global_quality.to_string());

                    options.push("-compression_level".to_string());

                    let level = if quality > 80 {
                        "1" // High quality, slower
                    } else if quality > 40 {
                        "4" // Medium
                    } else {
                        "7" // Low quality, faster
                    };

                    options.push(level.to_string());
                }
                HardwareAccelType::VideoToolbox => {
                    // VideoToolbox options
                    options.push("-b:v".to_string());
                    let bitrate = ((quality as f32 / 100.0) * 8000.0 + 2000.0).round() as u32;
                    options.push(format!("{}k", bitrate));

                    options.push("-profile:v".to_string());
                    options.push("high".to_string());
                }
                HardwareAccelType::Dxva2 => {
                    // DXVA2 only accelerates decoding, not encoding
                    // Fall back to software encoding options
                    options.push("-b:v".to_string());
                    let bitrate = ((quality as f32 / 100.0) * 6000.0 + 2000.0).round() as u32;
                    options.push(format!("{}k", bitrate));
                }
                _ => {
                    // Fallback to basic options
                    options.push("-b:v".to_string());
                    let bitrate = ((quality as f32 / 100.0) * 6000.0 + 2000.0).round() as u32;
                    options.push(format!("{}k", bitrate));
                }
            }
        }

        // Add options to limit GPU memory usage if specified
        if self.is_enabled() && self.max_gpu_memory.is_some() {
            let memory_mb = self.max_gpu_memory.unwrap() / (1024 * 1024);

            match self.selected_type {
                HardwareAccelType::Nvidia => {
                    options.push("-maxBufferSize".to_string());
                    options.push(memory_mb.to_string());
                }
                _ => {} // Other GPUs don't have direct memory control in FFmpeg
            }
        }

        options
    }

    /// Applies GPU acceleration settings to an FFmpeg command.
    ///
    /// # Arguments
    ///
    /// * `command` - The FFmpeg command to modify
    /// * `input_options` - Whether to add input options for decoding
    /// * `output_options` - Whether to add output options for encoding
    /// * `codec` - The video codec to use for encoding
    /// * `quality` - Quality setting (0-100) for the encoder
    ///
    /// # Returns
    ///
    /// The modified FFmpeg command with GPU acceleration settings
    pub fn apply_to_command<'a, 'b>(
        &'b self,
        command: &'a mut crate::ffmpeg::command::FFmpegCommand<'b>,
        input_options: bool,
        output_options: bool,
        codec: VideoCodec,
        quality: u32,
    ) -> &'a mut crate::ffmpeg::command::FFmpegCommand<'b> {
        // Do nothing if GPU acceleration is not enabled
        if !self.is_enabled() {
            return command;
        }

        // Add decoder options for hardware acceleration if requested
        if input_options {
            let decoder_options = self.get_decoder_options();
            for option in decoder_options {
                command.add_input_option(option, "");
            }
        }

        // Add encoder options for hardware acceleration if requested
        if output_options {
            // Set the hardware encoder
            let hw_encoder = self.get_encoder_name(codec);
            command.add_output_option("-c:v", &hw_encoder);

            // Add any encoder-specific options
            let encoder_options = self.get_encoder_options(codec, quality);
            for option in encoder_options {
                let parts: Vec<&str> = option.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    command.add_output_option(parts[0], parts[1]);
                } else if !option.is_empty() {
                    command.add_output_option(option, "");
                }
            }
        }

        command
    }
}

/// Check if a system has GPU acceleration support.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use for checking
///
/// # Returns
///
/// `true` if GPU acceleration is available, `false` otherwise.
pub fn has_gpu_acceleration(ffmpeg: &FFmpeg) -> bool {
    gpu_test::has_gpu_support(ffmpeg)
}

/// Create a GPU accelerator configured from the render config.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `config` - The render configuration
///
/// # Returns
///
/// A `Result` containing the configured GPU accelerator, or an error.
pub fn create_gpu_accelerator(
    ffmpeg: Arc<FFmpeg>,
    config: &RenderConfig,
) -> FFmpegResult<GpuAccelerator> {
    let mut accelerator = GpuAccelerator::new(ffmpeg);
    accelerator.initialize(config)?;
    Ok(accelerator)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be skipped by default as they require FFmpeg
    // To run them, use: cargo test -- --ignored

    #[test]
    #[ignore]
    fn test_gpu_accelerator_initialization() {
        if let Ok(ffmpeg) = FFmpeg::detect() {
            let ffmpeg = Arc::new(ffmpeg);
            let mut config = RenderConfig::default();
            config.hardware_accel_type = HardwareAccelType::Auto;
            config.use_hw_decoding = true;

            let result = create_gpu_accelerator(ffmpeg, &config);
            assert!(result.is_ok());

            let accelerator = result.unwrap();
            println!(
                "Selected acceleration: {:?}",
                accelerator.get_acceleration_type()
            );
            println!("Enabled: {}", accelerator.is_enabled());
        }
    }

    #[test]
    #[ignore]
    fn test_encoder_options() {
        if let Ok(ffmpeg) = FFmpeg::detect() {
            let ffmpeg = Arc::new(ffmpeg);
            let mut accelerator = GpuAccelerator::new(ffmpeg);

            // Test software encoding options
            let cpu_options = accelerator.get_encoder_options(VideoCodec::H264, 75);
            println!("CPU encoding options: {:?}", cpu_options);

            // Test with a GPU if available
            if let Ok(()) = accelerator.select_acceleration_type(HardwareAccelType::Nvidia) {
                let gpu_options = accelerator.get_encoder_options(VideoCodec::H264, 75);
                println!("GPU encoding options: {:?}", gpu_options);
            }
        }
    }
}
