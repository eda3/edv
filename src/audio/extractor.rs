/// Audio extraction functionality.
///
/// This module provides functions for extracting audio tracks from video files
/// into separate audio files with various format options.

use std::path::Path;

use crate::ffmpeg::{FFmpeg, command::FFmpegCommand};
use crate::audio::error::{Result, Error};
use crate::audio::common;

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

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            codec: common::DEFAULT_AUDIO_CODEC.to_string(),
            bitrate: common::DEFAULT_AUDIO_BITRATE.to_string(),
            sample_rate: common::DEFAULT_AUDIO_SAMPLE_RATE,
            channels: 2,
            stream_index: None,
            normalize_audio: false,
            start_time: None,
            duration: None,
        }
    }
}

impl ExtractionOptions {
    /// Creates a new instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the audio codec.
    #[must_use]
    pub fn codec(mut self, codec: &str) -> Self {
        let codec = codec.to_lowercase();
        if common::is_supported_audio_format(&codec) {
            self.codec = codec;
        }
        self
    }

    /// Sets the audio bitrate.
    ///
    /// # Arguments
    ///
    /// * `bitrate` - Bitrate as a string with unit (e.g., "192k", "320k")
    #[must_use]
    pub fn bitrate(mut self, bitrate: &str) -> Self {
        self.bitrate = bitrate.to_string();
        self
    }

    /// Sets the audio sample rate.
    #[must_use]
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Sets the number of audio channels.
    #[must_use]
    pub fn channels(mut self, channels: u8) -> Self {
        // Limit to reasonable values
        self.channels = channels.clamp(1, 8);
        self
    }

    /// Sets which audio stream to extract.
    #[must_use]
    pub fn stream_index(mut self, index: usize) -> Self {
        self.stream_index = Some(index);
        self
    }

    /// Sets whether to normalize audio during extraction.
    #[must_use]
    pub fn normalize_audio(mut self, normalize: bool) -> Self {
        self.normalize_audio = normalize;
        self
    }

    /// Sets the extraction start time.
    #[must_use]
    pub fn start_time(mut self, seconds: f64) -> Self {
        self.start_time = Some(seconds.max(0.0));
        self
    }

    /// Sets the extraction duration.
    #[must_use]
    pub fn duration(mut self, seconds: f64) -> Self {
        self.duration = Some(seconds.max(0.0));
        self
    }
}

/// Extracts audio from a video file using the specified options.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input video file
/// * `output` - Path to the output audio file
/// * `options` - Extraction options
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
pub fn extract_audio<P1, P2>(
    ffmpeg: FFmpeg,
    input: P1,
    output: P2,
    options: ExtractionOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let mut cmd = FFmpegCommand::new(ffmpeg);
    
    // Add input file with optional start time
    if let Some(start) = options.start_time {
        cmd.input_options(&["-ss", &start.to_string()])
           .input(input);
    } else {
        cmd.input(input);
    }
    
    // Prepare output options
    let mut output_opts = Vec::new();
    
    // Set duration if specified
    if let Some(duration) = options.duration {
        output_opts.push("-t");
        output_opts.push(&duration.to_string());
    }
    
    // Set audio codec
    output_opts.push("-c:a");
    output_opts.push(&options.codec);
    
    // Set audio bitrate
    output_opts.push("-b:a");
    output_opts.push(&options.bitrate);
    
    // Set sample rate
    output_opts.push("-ar");
    output_opts.push(&options.sample_rate.to_string());
    
    // Set channels
    output_opts.push("-ac");
    output_opts.push(&options.channels.to_string());
    
    // Extract specific audio stream if requested
    if let Some(index) = options.stream_index {
        output_opts.push("-map");
        output_opts.push(&format!("0:a:{}", index));
    } else {
        // Only extract audio streams
        output_opts.push("-vn");
    }
    
    // Apply audio normalization if requested
    if options.normalize_audio {
        output_opts.push("-filter:a");
        output_opts.push("loudnorm=I=-16:TP=-1.5:LRA=11");
    }
    
    // Finalize command
    cmd.output_options(&output_opts)
       .output(output)
       .overwrite(true);
    
    cmd.execute().map_err(Error::from)
}

/// Lists all audio streams in a video file.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input video file
///
/// # Returns
///
/// A Result containing a vector of stream information (index, codec, channels, sample rate)
///
/// # Errors
///
/// Returns an error if the file can't be processed
pub fn list_audio_streams<P>(
    ffmpeg: FFmpeg,
    input: P,
) -> Result<Vec<(usize, String, u8, u32)>>
where
    P: AsRef<Path>,
{
    // This is a placeholder for now, as it requires more complex implementation
    // that depends on the actual FFmpeg command execution and output parsing.
    // In a real implementation, you would need to:
    // 1. Run FFmpeg with -stats to get stream information
    // 2. Parse the output to extract audio stream details
    // 3. Return the structured information
    
    // For now, return an empty vector to avoid compilation errors
    Ok(Vec::new())
}

/// Extracts specific audio segments from a video and concatenates them.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input video file
/// * `output` - Path to the output audio file
/// * `segments` - List of (start_time, duration) tuples in seconds
/// * `options` - Extraction options
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
pub fn extract_segments<P1, P2>(
    ffmpeg: FFmpeg,
    input: P1,
    output: P2,
    segments: &[(f64, f64)],
    options: ExtractionOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    if segments.is_empty() {
        return Err(Error::ProcessingError("No segments specified".to_string()));
    }
    
    // Validate segments
    for (i, (start, duration)) in segments.iter().enumerate() {
        if *start < 0.0 || *duration <= 0.0 {
            return Err(Error::ProcessingError(
                format!("Invalid segment {}: start={}, duration={}", i, start, duration)
            ));
        }
    }
    
    // Build a complex filter for extracting and concatenating segments
    let mut filter_complex = String::new();
    
    // Create filter for each segment
    for (i, (start, duration)) in segments.iter().enumerate() {
        filter_complex.push_str(&format!(
            "[0:a]atrim=start={}:duration={},asetpts=PTS-STARTPTS[a{}];",
            start, duration, i
        ));
    }
    
    // Concatenate all segments
    filter_complex.push_str(&format!(
        "{}concat=n={}:v=0:a=1[outa]",
        segments.iter().enumerate().map(|(i, _)| format!("[a{}]", i)).collect::<Vec<_>>().join(""),
        segments.len()
    ));
    
    // Build the command
    let mut cmd = FFmpegCommand::new(ffmpeg);
    cmd.input(input)
       .filter_complex(&filter_complex)
       .output_options(&[
           "-map", "[outa]",
           "-c:a", &options.codec,
           "-b:a", &options.bitrate,
           "-ar", &options.sample_rate.to_string(),
           "-ac", &options.channels.to_string(),
       ])
       .output(output)
       .overwrite(true);
    
    cmd.execute().map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extraction_options_defaults() {
        let options = ExtractionOptions::default();
        assert_eq!(options.codec, common::DEFAULT_AUDIO_CODEC);
        assert_eq!(options.bitrate, common::DEFAULT_AUDIO_BITRATE);
        assert_eq!(options.sample_rate, common::DEFAULT_AUDIO_SAMPLE_RATE);
        assert_eq!(options.channels, 2);
        assert!(options.stream_index.is_none());
        assert!(!options.normalize_audio);
    }
    
    #[test]
    fn test_extraction_options_fluent_api() {
        let options = ExtractionOptions::new()
            .codec("mp3")
            .bitrate("320k")
            .sample_rate(48000)
            .channels(1)
            .stream_index(0)
            .normalize_audio(true)
            .start_time(10.5)
            .duration(30.0);
            
        assert_eq!(options.codec, "mp3");
        assert_eq!(options.bitrate, "320k");
        assert_eq!(options.sample_rate, 48000);
        assert_eq!(options.channels, 1);
        assert_eq!(options.stream_index, Some(0));
        assert!(options.normalize_audio);
        assert_eq!(options.start_time, Some(10.5));
        assert_eq!(options.duration, Some(30.0));
    }
    
    #[test]
    fn test_extraction_options_validation() {
        // Test channel clamping
        let options = ExtractionOptions::new().channels(12);
        assert_eq!(options.channels, 8); // Should be clamped to 8
        
        // Test start time validation
        let options = ExtractionOptions::new().start_time(-5.0);
        assert_eq!(options.start_time, Some(0.0)); // Should be clamped to 0
    }
} 