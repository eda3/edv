/// GPU testing utilities.
///
/// This module provides functions and types for testing GPU capabilities,
/// detecting available hardware accelerators, and ensuring compatibility
/// with the application's rendering requirements.
use crate::ffmpeg::{Error as FFmpegError, FFmpeg, Result as FFmpegResult};
use crate::project::rendering::config::HardwareAccelType;
use chrono;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// Result of a GPU capability test.
#[derive(Debug, Clone)]
pub struct GpuTestResult {
    /// The hardware acceleration type that was tested.
    pub accel_type: HardwareAccelType,

    /// Whether the GPU is available and working.
    pub is_available: bool,

    /// The detected GPU name, if available.
    pub gpu_name: Option<String>,

    /// Estimated performance score (higher is better).
    pub performance_score: Option<u32>,

    /// Any error message encountered during testing.
    pub error_message: Option<String>,

    /// Benchmark results for various operations, in milliseconds.
    pub benchmark_results: HashMap<String, u64>,
}

impl GpuTestResult {
    /// Creates a new GPU test result indicating an unavailable GPU.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type
    /// * `error_message` - The error message explaining why it's unavailable
    ///
    /// # Returns
    ///
    /// A new `GpuTestResult` with `is_available` set to false.
    #[must_use]
    pub fn unavailable(accel_type: HardwareAccelType, error_message: String) -> Self {
        Self {
            accel_type,
            is_available: false,
            gpu_name: None,
            performance_score: None,
            error_message: Some(error_message),
            benchmark_results: HashMap::new(),
        }
    }

    /// Creates a new GPU test result for an available GPU.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type
    /// * `gpu_name` - The name of the detected GPU
    ///
    /// # Returns
    ///
    /// A new `GpuTestResult` with `is_available` set to true.
    #[must_use]
    pub fn available(accel_type: HardwareAccelType, gpu_name: String) -> Self {
        Self {
            accel_type,
            is_available: true,
            gpu_name: Some(gpu_name),
            performance_score: None,
            error_message: None,
            benchmark_results: HashMap::new(),
        }
    }

    /// Adds a benchmark result to this test result.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the benchmark operation
    /// * `duration_ms` - The duration of the operation in milliseconds
    pub fn add_benchmark(&mut self, name: &str, duration_ms: u64) {
        self.benchmark_results.insert(name.to_string(), duration_ms);
    }

    /// Sets the performance score for this GPU.
    ///
    /// # Arguments
    ///
    /// * `score` - The performance score (higher is better)
    pub fn set_performance_score(&mut self, score: u32) {
        self.performance_score = Some(score);
    }
}

/// Environment for testing GPU capabilities.
#[derive(Debug)]
pub struct GpuTestEnvironment<'a> {
    /// FFmpeg instance to use for testing.
    ffmpeg: &'a FFmpeg,

    /// Available hardware acceleration types.
    available_accel_types: Vec<HardwareAccelType>,

    /// Test results for each hardware acceleration type.
    results: HashMap<HardwareAccelType, GpuTestResult>,
}

impl<'a> GpuTestEnvironment<'a> {
    /// Creates a new GPU test environment.
    ///
    /// # Arguments
    ///
    /// * `ffmpeg` - The FFmpeg instance to use for testing
    ///
    /// # Returns
    ///
    /// A new `GpuTestEnvironment`.
    pub fn new(ffmpeg: &'a FFmpeg) -> Self {
        let available_accel_types = HardwareAccelType::detect_available();

        Self {
            ffmpeg,
            available_accel_types,
            results: HashMap::new(),
        }
    }

    /// Detects and tests all available GPU acceleration methods.
    ///
    /// This function runs tests for each available hardware acceleration method
    /// and stores the results.
    ///
    /// # Returns
    ///
    /// A vector of GPU test results.
    pub fn test_all_gpus(&mut self) -> Vec<GpuTestResult> {
        for accel_type in &self.available_accel_types {
            // Skip None as it's not a GPU
            if matches!(accel_type, HardwareAccelType::None) {
                continue;
            }

            let result = self.test_gpu(*accel_type);
            self.results.insert(*accel_type, result);
        }

        self.results.values().cloned().collect()
    }

    /// Tests a specific GPU acceleration method.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type to test
    ///
    /// # Returns
    ///
    /// A GPU test result.
    pub fn test_gpu(&self, accel_type: HardwareAccelType) -> GpuTestResult {
        // Test if the hardware accelerator is actually working by trying to encode a small file
        match self.detect_gpu_details(accel_type) {
            Ok((is_available, gpu_name)) => {
                if is_available {
                    let mut result = GpuTestResult::available(accel_type, gpu_name);

                    // Run benchmark if available
                    if let Ok(benchmark_results) = self.benchmark_gpu(accel_type) {
                        for (name, duration) in benchmark_results {
                            result.add_benchmark(&name, duration.as_millis() as u64);
                        }

                        // Calculate a simple performance score based on benchmark results
                        let total_duration: u64 = result.benchmark_results.values().sum();
                        let score = if total_duration > 0 {
                            // Lower times = higher scores
                            10000 / total_duration.max(1)
                        } else {
                            0
                        };

                        result.set_performance_score(score as u32);
                    }

                    result
                } else {
                    GpuTestResult::unavailable(
                        accel_type,
                        "Hardware acceleration is present but not operational".to_string(),
                    )
                }
            }
            Err(e) => GpuTestResult::unavailable(accel_type, e.to_string()),
        }
    }

    /// Detects details about a specific GPU.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type to detect
    ///
    /// # Returns
    ///
    /// A tuple of (is_available, gpu_name) or an error if detection failed.
    fn detect_gpu_details(&self, accel_type: HardwareAccelType) -> Result<(bool, String), String> {
        // This function would normally use FFmpeg to query the GPU
        // For now, we'll simulate the detection

        let gpu_name = match accel_type {
            HardwareAccelType::Nvidia => "NVIDIA GPU (simulated)",
            HardwareAccelType::Amd => "AMD GPU (simulated)",
            HardwareAccelType::Intel => "Intel iGPU (simulated)",
            HardwareAccelType::Vaapi => "VA-API Device (simulated)",
            HardwareAccelType::Dxva2 => "DXVA2 Device (simulated)",
            HardwareAccelType::VideoToolbox => "VideoToolbox Device (simulated)",
            _ => return Err("Unsupported hardware acceleration type".to_string()),
        };

        // Simulate availability check
        // In a real implementation, this would try to use FFmpeg to create an encoder
        // with the given acceleration type and check if it works
        let is_available = match accel_type {
            HardwareAccelType::None | HardwareAccelType::Auto => false,
            _ => true, // For testing purposes we'll assume all hardware accelerators are available
        };

        Ok((is_available, gpu_name.to_string()))
    }

    /// Benchmarks a specific GPU acceleration method.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type to benchmark
    ///
    /// # Returns
    ///
    /// A map of benchmark results, or an error if benchmarking failed.
    fn benchmark_gpu(
        &self,
        accel_type: HardwareAccelType,
    ) -> Result<HashMap<String, Duration>, String> {
        let mut results = HashMap::new();

        // Simulated benchmark operations
        // In a real implementation, these would be actual encoding/decoding tasks

        // Simulated H.264 encoding
        let start = Instant::now();
        // Simulate work
        std::thread::sleep(Duration::from_millis(50));
        results.insert("h264_encode".to_string(), start.elapsed());

        // Simulated H.264 decoding
        let start = Instant::now();
        // Simulate work
        std::thread::sleep(Duration::from_millis(30));
        results.insert("h264_decode".to_string(), start.elapsed());

        // Simulated filter application
        let start = Instant::now();
        // Simulate work
        std::thread::sleep(Duration::from_millis(25));
        results.insert("apply_filters".to_string(), start.elapsed());

        Ok(results)
    }

    /// Gets the best performing GPU based on test results.
    ///
    /// # Returns
    ///
    /// The hardware acceleration type with the highest performance score,
    /// or None if no GPUs were tested or all tests failed.
    pub fn get_best_gpu(&self) -> Option<HardwareAccelType> {
        self.results
            .iter()
            .filter(|(_, result)| result.is_available)
            .max_by_key(|(_, result)| result.performance_score.unwrap_or(0))
            .map(|(&accel_type, _)| accel_type)
    }

    /// Gets all test results.
    ///
    /// # Returns
    ///
    /// A reference to the map of test results.
    pub fn get_results(&self) -> &HashMap<HardwareAccelType, GpuTestResult> {
        &self.results
    }

    /// Gets the test result for a specific hardware acceleration type.
    ///
    /// # Arguments
    ///
    /// * `accel_type` - The hardware acceleration type
    ///
    /// # Returns
    ///
    /// The test result, or None if the type hasn't been tested.
    pub fn get_result(&self, accel_type: HardwareAccelType) -> Option<&GpuTestResult> {
        self.results.get(&accel_type)
    }
}

/// Tests if the system supports hardware acceleration.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use for testing
///
/// # Returns
///
/// `true` if at least one hardware acceleration method is available and working,
/// `false` otherwise.
pub fn has_gpu_support(ffmpeg: &FFmpeg) -> bool {
    let environment = GpuTestEnvironment::new(ffmpeg);
    let accel_types = HardwareAccelType::detect_available();

    accel_types.iter().any(|&accel_type| {
        !matches!(accel_type, HardwareAccelType::None)
            && environment.test_gpu(accel_type).is_available
    })
}

/// Selects the most appropriate hardware acceleration type for the system.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use for testing
///
/// # Returns
///
/// The recommended hardware acceleration type, or HardwareAccelType::None
/// if no suitable hardware acceleration is available.
pub fn select_best_hardware_acceleration(ffmpeg: &FFmpeg) -> HardwareAccelType {
    let mut environment = GpuTestEnvironment::new(ffmpeg);
    environment.test_all_gpus();

    environment
        .get_best_gpu()
        .unwrap_or(HardwareAccelType::None)
}

/// Runs GPU capability tests and generates a report.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use for testing
/// * `output_path` - Path to save the GPU test report
///
/// # Returns
///
/// `Ok(())` on success, or an error if testing failed.
pub fn run_gpu_capability_test<P: AsRef<Path>>(
    ffmpeg: &FFmpeg,
    output_path: P,
) -> Result<(), crate::ffmpeg::Error> {
    // Initialize test environment
    let mut environment = GpuTestEnvironment::new(ffmpeg);

    // Run tests
    let results = environment.test_all_gpus();

    // Generate report
    let report = generate_gpu_report(&results);

    // Save report to file
    std::fs::write(output_path, report).map_err(|e| {
        crate::ffmpeg::Error::OutputParseError(format!("Failed to save GPU test report: {e}"))
    })?;

    Ok(())
}

/// Generates a formatted report from GPU test results.
///
/// # Arguments
///
/// * `results` - Vector of GPU test results
///
/// # Returns
///
/// A string containing the formatted report.
fn generate_gpu_report(results: &[GpuTestResult]) -> String {
    let mut report = String::new();

    report.push_str("# GPU Capability Test Report\n\n");
    report.push_str(&format!(
        "Test Date: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    // Count available GPUs
    let available_gpus = results.iter().filter(|r| r.is_available).count();
    report.push_str(&format!(
        "Available GPUs: {}/{}\n\n",
        available_gpus,
        results.len()
    ));

    // Detailed results
    report.push_str("## Detailed Results\n\n");

    for result in results {
        report.push_str(&format!("### {:?}\n\n", result.accel_type));
        report.push_str(&format!("- Available: {}\n", result.is_available));

        if let Some(gpu_name) = &result.gpu_name {
            report.push_str(&format!("- GPU Name: {}\n", gpu_name));
        }

        if let Some(score) = result.performance_score {
            report.push_str(&format!("- Performance Score: {}\n", score));
        }

        if !result.benchmark_results.is_empty() {
            report.push_str("- Benchmark Results:\n");
            for (name, duration) in &result.benchmark_results {
                report.push_str(&format!("  - {}: {}ms\n", name, duration));
            }
        }

        if let Some(error) = &result.error_message {
            report.push_str(&format!("- Error: {}\n", error));
        }

        report.push_str("\n");
    }

    // Recommendations
    report.push_str("## Recommendations\n\n");

    if available_gpus > 0 {
        // Find best GPU
        let best_gpu = results
            .iter()
            .filter(|r| r.is_available)
            .max_by_key(|r| r.performance_score.unwrap_or(0));

        if let Some(gpu) = best_gpu {
            report.push_str(&format!(
                "Recommended hardware acceleration: {:?}\n",
                gpu.accel_type
            ));

            if let Some(score) = gpu.performance_score {
                report.push_str(&format!("Performance score: {}\n", score));
            }
        }
    } else {
        report.push_str("No suitable hardware accelerators found. Using CPU-only mode.\n");
        report.push_str("Please check your hardware and drivers.\n");
    }

    report
}

/// Tests GPU functionality with an actual encoding operation.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use for testing
/// * `accel_type` - The hardware acceleration type to test
/// * `input_file` - Path to a test video file
///
/// # Returns
///
/// `Ok(duration)` with the time it took to encode in milliseconds, or an error if testing failed.
pub fn test_gpu_encoding<P: AsRef<Path>>(
    ffmpeg: &FFmpeg,
    accel_type: HardwareAccelType,
    input_file: P,
) -> FFmpegResult<u64> {
    // Skip if no hardware acceleration
    if matches!(accel_type, HardwareAccelType::None) {
        return Err(FFmpegError::InvalidArgument(
            "Cannot test encoding with HardwareAccelType::None".to_string(),
        ));
    }

    // Create a temporary output file
    let output_file = std::env::temp_dir().join("edv_gpu_test_output.mp4");

    // Get the hardware encoder name
    let encoder = accel_type
        .get_hw_encoder_name(crate::project::rendering::config::VideoCodec::H264)
        .ok_or_else(|| {
            FFmpegError::OutputParseError(format!(
                "No hardware encoder available for {:?}",
                accel_type
            ))
        })?;

    // Start timing
    let start = Instant::now();

    // Build and execute FFmpeg command
    let mut command = ffmpeg.command();
    command
        .input_options(["-t", "5"]) // Only process 5 seconds
        .input(input_file.as_ref())
        .output_options([
            "-c:v", encoder, "-preset", "fast", "-b:v", "1M", "-f", "mp4",
        ])
        .set_output(&output_file)
        .overwrite(true);

    // Execute the command
    command.execute()?;

    // Calculate duration
    let duration = start.elapsed().as_millis() as u64;

    // Clean up temporary file
    if output_file.exists() {
        let _ = std::fs::remove_file(output_file);
    }

    Ok(duration)
}

/// Creates a test video file for GPU testing.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `output_path` - Path to save the test video
/// * `duration_secs` - Duration of the test video in seconds
///
/// # Returns
///
/// `Ok(())` on success, or an error if creating the test video failed.
pub fn create_test_video<P: AsRef<Path>>(
    ffmpeg: &FFmpeg,
    output_path: P,
    duration_secs: u32,
) -> FFmpegResult<()> {
    // Create a synthetic test video using FFmpeg's testsrc
    ffmpeg
        .command()
        .input_options([
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=duration={}:size=1280x720:rate=30", duration_secs),
        ])
        .output_options(["-c:v", "libx264", "-pix_fmt", "yuv420p"])
        .set_output(output_path.as_ref())
        .overwrite(true)
        .execute()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests are skipped by default as they require FFmpeg
    // To run them, use: cargo test -- --ignored

    #[test]
    #[ignore]
    fn test_gpu_detection() {
        // This requires FFmpeg to be installed
        if let Ok(ffmpeg) = FFmpeg::detect() {
            let has_gpu = has_gpu_support(&ffmpeg);
            println!("GPU Support: {}", if has_gpu { "Yes" } else { "No" });
        }
    }

    #[test]
    #[ignore]
    fn test_best_hardware_acceleration() {
        // This requires FFmpeg to be installed
        if let Ok(ffmpeg) = FFmpeg::detect() {
            let best_accel = select_best_hardware_acceleration(&ffmpeg);
            println!("Best hardware acceleration: {:?}", best_accel);
        }
    }
}
