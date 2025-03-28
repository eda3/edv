/// Audio fade-in and fade-out effects.
///
/// This module provides functions for applying fade-in and fade-out effects to audio
/// tracks in video files, allowing for smooth transitions.
use std::path::Path;

use crate::audio::common;
use crate::audio::error::{Error, Result};
use crate::ffmpeg::{FFmpeg, command::FFmpegCommand};

/// Types of audio fade effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FadeType {
    /// Linear fade (constant amplitude change)
    Linear,

    /// Exponential fade (logarithmic amplitude change, sounds more natural)
    Exponential,

    /// Sinusoidal (sine curve-based fade)
    Sinusoidal,

    /// Logarithmic fade
    Logarithmic,
}

impl FadeType {
    /// Converts the fade type to a string recognized by FFmpeg.
    #[must_use]
    fn as_ffmpeg_string(&self) -> &'static str {
        match self {
            Self::Linear => "t",
            Self::Exponential => "exp",
            Self::Sinusoidal => "sin",
            Self::Logarithmic => "log",
        }
    }
}

/// Fade operation parameters.
#[derive(Debug, Clone)]
pub struct FadeOptions {
    /// Whether to preserve the video quality (no re-encoding)
    pub preserve_video: bool,

    /// The audio codec to use for output
    pub audio_codec: String,

    /// The bitrate for output audio
    pub audio_bitrate: String,

    /// The sample rate for output audio
    pub sample_rate: u32,
}

impl Default for FadeOptions {
    fn default() -> Self {
        Self {
            preserve_video: true,
            audio_codec: common::DEFAULT_AUDIO_CODEC.to_string(),
            audio_bitrate: common::DEFAULT_AUDIO_BITRATE.to_string(),
            sample_rate: common::DEFAULT_SAMPLE_RATE,
        }
    }
}

impl FadeOptions {
    /// Creates a new instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to preserve the video quality.
    #[must_use]
    pub fn preserve_video(mut self, preserve: bool) -> Self {
        self.preserve_video = preserve;
        self
    }

    /// Sets the audio codec.
    #[must_use]
    pub fn audio_codec(mut self, codec: &str) -> Self {
        let codec = codec.to_lowercase();
        if common::is_supported_format(&codec) {
            self.audio_codec = codec;
        }
        self
    }

    /// Sets the audio bitrate for the output.
    #[must_use]
    pub fn audio_bitrate(mut self, bitrate: &str) -> Self {
        self.audio_bitrate = bitrate.to_string();
        self
    }

    /// Sets the audio sample rate for the output.
    #[must_use]
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }
}

/// Applies a fade-in effect to the beginning of the audio.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `duration` - Duration of the fade-in effect in seconds
/// * `fade_type` - Type of fade curve to apply
/// * `options` - Additional options for the operation
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
pub fn fade_in<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    duration: f64,
    fade_type: FadeType,
    options: &FadeOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    if duration <= 0.0 {
        return Err(Error::InvalidFadeDuration(duration));
    }

    let curve = fade_type.as_ffmpeg_string();
    let fade_filter = format!("afade=t=in:st=0:d={duration}:curve={curve}");

    let mut cmd = FFmpegCommand::new(ffmpeg);

    // 出力オプションを所有型の文字列で構築
    let output_options = vec![
        "-filter:a".to_string(),
        fade_filter,
        "-c:a".to_string(),
        options.audio_codec.clone(),
        "-b:a".to_string(),
        options.audio_bitrate.clone(),
        "-ar".to_string(),
        options.sample_rate.to_string(),
    ];

    cmd.input(input).output_options(&output_options);

    if options.preserve_video {
        let video_options = vec!["-c:v".to_string(), "copy".to_string()];
        cmd.output_options(&video_options);
    }

    cmd.output(output).overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Applies a fade-out effect to the end of the audio.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `duration` - Duration of the fade-out effect in seconds
/// * `fade_type` - Type of fade curve to apply
/// * `options` - Additional options for the operation
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
pub fn fade_out<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    duration: f64,
    fade_type: FadeType,
    options: &FadeOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    if duration <= 0.0 {
        return Err(Error::InvalidFadeDuration(duration));
    }

    // Get the duration of the input file
    let file_duration = get_duration(ffmpeg, input.as_ref())?;

    let start_time = file_duration - duration;
    if start_time < 0.0 {
        return Err(Error::InvalidFadeDuration(duration));
    }

    let curve = fade_type.as_ffmpeg_string();
    let fade_filter = format!("afade=t=out:st={start_time}:d={duration}:curve={curve}");

    let mut cmd = FFmpegCommand::new(ffmpeg);

    // 出力オプションを所有型の文字列で構築
    let output_options = vec![
        "-filter:a".to_string(),
        fade_filter,
        "-c:a".to_string(),
        options.audio_codec.clone(),
        "-b:a".to_string(),
        options.audio_bitrate.clone(),
        "-ar".to_string(),
        options.sample_rate.to_string(),
    ];

    cmd.input(input).output_options(&output_options);

    if options.preserve_video {
        let video_options = vec!["-c:v".to_string(), "copy".to_string()];
        cmd.output_options(&video_options);
    }

    cmd.output(output).overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Applies both fade-in and fade-out effects to the audio.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `fade_in_duration` - Duration of the fade-in effect in seconds
/// * `fade_out_duration` - Duration of the fade-out effect in seconds
/// * `fade_type` - Type of fade curve to apply
/// * `options` - Additional options for the operation
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
pub fn fade_both<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    fade_in_duration: f64,
    fade_out_duration: f64,
    fade_type: FadeType,
    options: &FadeOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    if fade_in_duration <= 0.0 || fade_out_duration <= 0.0 {
        return Err(Error::InvalidFadeDuration(
            fade_in_duration.min(fade_out_duration),
        ));
    }

    // Get the duration of the input file
    let file_duration = get_duration(ffmpeg, input.as_ref())?;

    let fade_out_start = file_duration - fade_out_duration;
    if fade_out_start <= fade_in_duration {
        return Err(Error::ProcessingError(format!(
            "File is too short ({file_duration}s) for both fade-in ({fade_in_duration}s) and fade-out ({fade_out_duration}s)"
        )));
    }

    // Build both fade operations in one filter
    let curve = fade_type.as_ffmpeg_string();
    let fade_filter = format!(
        "afade=t=in:st=0:d={fade_in_duration}:curve={curve},afade=t=out:st={fade_out_start}:d={fade_out_duration}:curve={curve}"
    );

    let mut cmd = FFmpegCommand::new(ffmpeg);

    // 出力オプションを所有型の文字列で構築
    let output_options = vec![
        "-filter:a".to_string(),
        fade_filter,
        "-c:a".to_string(),
        options.audio_codec.clone(),
        "-b:a".to_string(),
        options.audio_bitrate.clone(),
        "-ar".to_string(),
        options.sample_rate.to_string(),
    ];

    cmd.input(input).output_options(&output_options);

    if options.preserve_video {
        let video_options = vec!["-c:v".to_string(), "copy".to_string()];
        cmd.output_options(&video_options);
    }

    cmd.output(output).overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Applies fade effects to specific segments of the audio.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `segments` - List of (start_time, duration, fade_in, fade_out) where fade_in and fade_out are booleans
/// * `fade_duration` - Duration for both fade-in and fade-out effects in seconds
/// * `fade_type` - Type of fade curve to apply
/// * `options` - Additional options for the operation
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
pub fn fade_segments<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    segments: &[(f64, f64, bool, bool)],
    fade_duration: f64,
    fade_type: FadeType,
    options: &FadeOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    if segments.is_empty() {
        return Err(Error::ProcessingError("No segments specified".to_string()));
    }

    if fade_duration <= 0.0 {
        return Err(Error::InvalidFadeDuration(fade_duration));
    }

    // Build a complex filter for applying fades to segments
    let mut filter_complex = String::new();

    // Create filter for each segment
    for (i, (start, duration, fade_in, fade_out)) in segments.iter().enumerate() {
        if *start < 0.0 || *duration <= 0.0 {
            return Err(Error::ProcessingError(format!(
                "Invalid segment {i}: start={start}, duration={duration}"
            )));
        }

        if *fade_in && *fade_out && *duration <= 2.0 * fade_duration {
            return Err(Error::ProcessingError(format!(
                "Segment {i} is too short for both fade-in and fade-out"
            )));
        }

        filter_complex.push_str(&format!(
            "[0:a]atrim=start={start}:duration={duration},asetpts=PTS-STARTPTS"
        ));

        // Apply fade-in if requested
        if *fade_in {
            let curve = fade_type.as_ffmpeg_string();
            filter_complex.push_str(&format!(",afade=t=in:st=0:d={fade_duration}:curve={curve}"));
        }

        // Apply fade-out if requested
        if *fade_out {
            let fade_start = duration - fade_duration;
            let curve = fade_type.as_ffmpeg_string();
            filter_complex.push_str(&format!(
                ",afade=t=out:st={fade_start}:d={fade_duration}:curve={curve}"
            ));
        }

        filter_complex.push_str(&format!("[a{i}];"));
    }

    // Concatenate all segments
    let segment_refs = segments
        .iter()
        .enumerate()
        .map(|(i, _)| format!("[a{i}]"))
        .collect::<Vec<_>>()
        .join("");
    let segment_count = segments.len();

    filter_complex.push_str(&format!(
        "{segment_refs}concat=n={segment_count}:v=0:a=1[outa]"
    ));

    // Build the command
    let mut cmd = FFmpegCommand::new(ffmpeg);

    // 出力オプションを所有型の文字列で構築
    let audio_options = vec![
        "-map".to_string(),
        "[outa]".to_string(),
        "-c:a".to_string(),
        options.audio_codec.clone(),
        "-b:a".to_string(),
        options.audio_bitrate.clone(),
        "-ar".to_string(),
        options.sample_rate.to_string(),
    ];

    cmd.input(input)
        .filter_complex(&filter_complex)
        .output_options(&audio_options);

    if options.preserve_video {
        let video_options = vec![
            "-map".to_string(),
            "0:v".to_string(),
            "-c:v".to_string(),
            "copy".to_string(),
        ];
        cmd.output_options(&video_options);
    }

    cmd.output(output).overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Helper function to get the duration of a media file.
fn get_duration<P>(_ffmpeg: &FFmpeg, _file: P) -> Result<f64>
where
    P: AsRef<Path>,
{
    // This is a placeholder. In a real implementation, you would:
    // 1. Run ffprobe to get file duration
    // 2. Parse the output to extract the duration value
    // 3. Return the duration in seconds

    // For now, we'll return a dummy value
    // In a real implementation, you would remove this and implement proper duration detection
    Ok(60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_type_conversions() {
        assert_eq!(FadeType::Linear.as_ffmpeg_string(), "t");
        assert_eq!(FadeType::Exponential.as_ffmpeg_string(), "exp");
        assert_eq!(FadeType::Sinusoidal.as_ffmpeg_string(), "sin");
        assert_eq!(FadeType::Logarithmic.as_ffmpeg_string(), "log");
    }

    #[test]
    fn test_fade_options_defaults() {
        let options = FadeOptions::default();
        assert!(options.preserve_video);
        assert_eq!(options.audio_codec, common::DEFAULT_AUDIO_CODEC);
        assert_eq!(options.audio_bitrate, common::DEFAULT_AUDIO_BITRATE);
        assert_eq!(options.sample_rate, common::DEFAULT_SAMPLE_RATE);
    }

    #[test]
    fn test_fade_options_fluent_api() {
        let options = FadeOptions::new()
            .preserve_video(false)
            .audio_codec("mp3")
            .audio_bitrate("320k")
            .sample_rate(48000);

        assert!(!options.preserve_video);
        assert_eq!(options.audio_codec, "mp3");
        assert_eq!(options.audio_bitrate, "320k");
        assert_eq!(options.sample_rate, 48000);
    }
}
