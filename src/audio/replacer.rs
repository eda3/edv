/// Audio replacement functionality.
///
/// This module provides functions for replacing audio tracks in video files
/// with audio from other files or generated silence.
use std::path::Path;

use crate::audio::common;
use crate::audio::error::{Error, Result};
use crate::ffmpeg::{FFmpeg, command::FFmpegCommand};

/// Audio replacement options.
#[derive(Debug, Clone)]
pub struct ReplacementOptions {
    /// The audio codec to use for the output
    pub codec: String,

    /// The bitrate for the output audio (e.g., "192k")
    pub bitrate: String,

    /// The sample rate for the output audio (e.g., 44100)
    pub sample_rate: u32,

    /// Offset in seconds for the replacement audio
    pub offset: f64,

    /// Volume adjustment for the replacement audio (1.0 = original volume)
    pub volume: f64,

    /// Mix original audio at this volume level (0.0 = no original audio)
    pub original_volume: f64,

    /// Ensure audio and video durations match
    pub match_duration: bool,

    /// Loop the replacement audio to match video duration
    pub loop_audio: bool,

    /// The number of channels for the output audio
    pub channels: u32,

    /// The stream index for the replacement audio
    pub stream_index: Option<u32>,

    /// Whether to normalize the audio
    pub normalize_audio: bool,

    /// The audio volume for the replacement audio
    pub audio_volume: f64,

    /// The video start time for the replacement audio
    pub video_start: Option<f64>,

    /// The audio start time for the replacement audio
    pub audio_start: Option<f64>,

    /// The duration for the replacement audio
    pub duration: Option<f64>,
}

impl Default for ReplacementOptions {
    fn default() -> Self {
        Self {
            codec: common::DEFAULT_AUDIO_CODEC.to_string(),
            bitrate: common::DEFAULT_AUDIO_BITRATE.to_string(),
            sample_rate: common::DEFAULT_SAMPLE_RATE,
            channels: 2,
            stream_index: None,
            normalize_audio: false,
            audio_volume: 1.0,
            video_start: None,
            audio_start: None,
            duration: None,
            offset: 0.0,
            volume: 1.0,
            original_volume: 0.0,
            match_duration: true,
            loop_audio: false,
        }
    }
}

impl ReplacementOptions {
    /// Creates a new instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the audio codec for the output.
    #[must_use]
    pub fn codec(mut self, codec: &str) -> Self {
        let codec = codec.to_lowercase();
        if common::is_supported_format(&codec) {
            self.codec = codec;
        }
        self
    }

    /// Sets the audio bitrate for the output.
    #[must_use]
    pub fn bitrate(mut self, bitrate: &str) -> Self {
        self.bitrate = bitrate.to_string();
        self
    }

    /// Sets the audio sample rate for the output.
    #[must_use]
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Sets an offset in seconds for the replacement audio.
    #[must_use]
    pub fn offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the volume adjustment for the replacement audio.
    #[must_use]
    pub fn volume(mut self, volume: f64) -> Self {
        self.volume = common::normalize_volume_level(volume);
        self
    }

    /// Sets the volume level for the original audio (for mixing).
    #[must_use]
    pub fn original_volume(mut self, volume: f64) -> Self {
        self.original_volume = common::normalize_volume_level(volume);
        self
    }

    /// Sets whether to ensure audio and video durations match.
    #[must_use]
    pub fn match_duration(mut self, match_duration: bool) -> Self {
        self.match_duration = match_duration;
        self
    }

    /// Sets whether to loop the replacement audio to match video duration.
    #[must_use]
    pub fn loop_audio(mut self, loop_audio: bool) -> Self {
        self.loop_audio = loop_audio;
        self
    }

    /// Sets the number of channels for the output audio.
    #[must_use]
    pub fn channels(mut self, channels: u32) -> Self {
        self.channels = channels;
        self
    }

    /// Sets the stream index for the replacement audio.
    #[must_use]
    pub fn stream_index(mut self, stream_index: Option<u32>) -> Self {
        self.stream_index = stream_index;
        self
    }

    /// Sets whether to normalize the audio.
    #[must_use]
    pub fn normalize_audio(mut self, normalize_audio: bool) -> Self {
        self.normalize_audio = normalize_audio;
        self
    }

    /// Sets the audio volume for the replacement audio.
    #[must_use]
    pub fn audio_volume(mut self, audio_volume: f64) -> Self {
        self.audio_volume = common::normalize_volume_level(audio_volume);
        self
    }

    /// Sets the video start time for the replacement audio.
    #[must_use]
    pub fn video_start(mut self, video_start: Option<f64>) -> Self {
        self.video_start = video_start;
        self
    }

    /// Sets the audio start time for the replacement audio.
    #[must_use]
    pub fn audio_start(mut self, audio_start: Option<f64>) -> Self {
        self.audio_start = audio_start;
        self
    }

    /// Sets the duration for the replacement audio.
    #[must_use]
    pub fn duration(mut self, duration: Option<f64>) -> Self {
        self.duration = duration;
        self
    }
}

/// Replaces the audio track in a video file with audio from another file.
///
/// # Arguments
///
/// * `ffmpeg` - The `FFmpeg` instance to use
/// * `video` - Path to the input video file
/// * `audio` - Path to the replacement audio file
/// * `output` - Path to the output video file
/// * `options` - Replacement options
///
/// # Returns
///
/// A `Result` indicating success or an error
///
/// # Errors
///
/// Returns an error if the files can't be processed
pub fn replace_audio<P1, P2, P3>(
    ffmpeg: &FFmpeg,
    video: P1,
    audio: P2,
    output: P3,
    options: &ReplacementOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let mut cmd = FFmpegCommand::new(ffmpeg);

    // Add input video
    cmd.input(video);

    // Add replacement audio with offset if needed
    if options.offset.abs() > 0.0 {
        cmd.input_options(["-ss", &options.offset.to_string()])
            .input(audio);
    } else {
        cmd.input(audio);
    }

    // Build the filter complex string based on options
    let mut filter = String::new();

    // First handle the replacement audio (potentially with looping)
    if options.loop_audio && options.match_duration {
        // Loop the audio to match the video duration
        filter.push_str("[1:a]aloop=loop=-1:size=2048[looped];");
        filter.push_str("[looped]");
    } else {
        filter.push_str("[1:a]");
    }

    // Apply volume adjustment to replacement audio if needed
    if (options.volume - 1.0).abs() > f64::EPSILON {
        let volume = options.volume;
        filter.push_str(&format!("volume={volume}"));
    }

    // Create a label for the adjusted replacement audio
    filter.push_str("[adjusted_audio];");

    // If we want to mix in the original audio
    if options.original_volume > f64::EPSILON {
        filter.push_str("[0:a]");
        let original_volume = options.original_volume;
        filter.push_str(&format!("volume={original_volume}"));
        filter.push_str("[adjusted_original];");

        // Mix the two audio streams
        filter.push_str(
            "[adjusted_audio][adjusted_original]amix=inputs=2:duration=first[final_audio]",
        );
    } else {
        // Just use the replacement audio
        filter.push_str("[adjusted_audio]amix=inputs=1[final_audio]");
    }

    // Build the output command
    cmd.filter_complex(&filter)
        .output_options([
            "-map",
            "0:v", // Take video from the first input (original video)
            "-map",
            "[final_audio]", // Use our processed audio
            "-c:v",
            "copy", // Copy video codec (no re-encoding)
            "-c:a",
            &options.codec, // Use specified audio codec
            "-b:a",
            &options.bitrate, // Use specified audio bitrate
            "-ar",
            &options.sample_rate.to_string(), // Use specified sample rate
        ])
        .set_output(output)
        .overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Replaces the audio track in a video file with silence.
///
/// # Arguments
///
/// * `ffmpeg` - The `FFmpeg` instance to use
/// * `input` - Path to the input video file
/// * `output` - Path to the output video file
/// * `options` - Replacement options (only `codec`, `bitrate`, and `sample_rate` are used)
///
/// # Returns
///
/// A `Result` indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed
pub fn replace_with_silence<P1, P2>(
    ffmpeg: &FFmpeg,
    input: P1,
    output: P2,
    options: &ReplacementOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let mut cmd = FFmpegCommand::new(ffmpeg);

    // Add input video
    cmd.input(input)
        .filter_complex("[0:a]aformat=channel_layouts=stereo,volume=0[silence]")
        .output_options([
            "-map",
            "0:v", // Take video from the input
            "-map",
            "[silence]", // Use generated silence
            "-c:v",
            "copy", // Copy video codec (no re-encoding)
            "-c:a",
            &options.codec, // Use specified audio codec
            "-b:a",
            &options.bitrate, // Use specified audio bitrate
            "-ar",
            &options.sample_rate.to_string(), // Use specified sample rate
        ])
        .set_output(output)
        .overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Replaces specific segments of audio in a video with audio from another file.
///
/// # Arguments
///
/// * `ffmpeg` - The `FFmpeg` instance to use
/// * `video` - Path to the input video file
/// * `audio` - Path to the replacement audio file
/// * `output` - Path to the output video file
/// * `segments` - List of (`video_start`, `video_end`, `audio_start`) positions in seconds
/// * `options` - Replacement options
///
/// # Returns
///
/// A `Result` indicating success or an error
///
/// # Errors
///
/// Returns an error if the files can't be processed
pub fn replace_segments<P1, P2, P3>(
    ffmpeg: &FFmpeg,
    video: P1,
    audio: P2,
    output: P3,
    segments: &[(f64, f64, f64)],
    options: &ReplacementOptions,
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    if segments.is_empty() {
        return Err(Error::ProcessingError("No segments specified".to_string()));
    }

    // Validate segments
    for (i, (v_start, v_end, a_start)) in segments.iter().enumerate() {
        if *v_start < 0.0 || *v_end <= *v_start || *a_start < 0.0 {
            return Err(Error::ProcessingError(format!(
                "Invalid segment {i}: video_start={v_start}, video_end={v_end}, audio_start={a_start}"
            )));
        }
    }

    let mut cmd = FFmpegCommand::new(ffmpeg);

    // Add input video and audio
    cmd.input(video).input(audio);

    // Build complex filter for segment replacement
    let mut filter = String::new();

    // Split original audio into segments
    filter.push_str("[0:a]asplit=");
    filter.push_str(&format!(
        "{segments_len}[original]",
        segments_len = segments.len() + 1
    ));
    for i in 0..segments.len() {
        filter.push_str(&format!("[seg{i}]"));
    }
    filter.push(';');

    // Process each segment for replacement
    for (i, (v_start, v_end, a_start)) in segments.iter().enumerate() {
        // Get segment from original audio
        filter.push_str(&format!(
            "[seg{i}]atrim=start={v_start}:end={v_end},asetpts=PTS-STARTPTS[oseg{i}];"
        ));

        // Get corresponding segment from replacement audio
        let a_duration = v_end - v_start;
        filter.push_str(&format!(
            "[1:a]atrim=start={a_start}:duration={a_duration},asetpts=PTS-STARTPTS"
        ));

        // Apply volume adjustment if needed
        if (options.volume - 1.0).abs() > f64::EPSILON {
            let volume = options.volume;
            filter.push_str(&format!(",volume={volume}"));
        }

        filter.push_str(&format!("[rseg{i}];"));
    }

    // Now build audio timeline by concatenating segments
    let mut concat_parts = Vec::new();
    let mut input_count = 0;

    // Process segments in order
    let mut last_end = 0.0;
    for (i, (v_start, v_end, _)) in segments.iter().enumerate() {
        // If there's a gap before this segment, add the original audio
        if *v_start > last_end {
            filter.push_str(&format!(
                "[original]atrim=start={last_end}:end={v_start},asetpts=PTS-STARTPTS[gap{i}];"
            ));
            concat_parts.push(format!("[gap{i}]"));
            input_count += 1;
        }

        // Add the replacement segment
        concat_parts.push(format!("[rseg{i}]"));
        input_count += 1;

        last_end = *v_end;
    }

    // If there's audio after the last segment, add it
    filter.push_str(&format!(
        "[original]atrim=start={last_end},asetpts=PTS-STARTPTS[remainder];"
    ));
    concat_parts.push("[remainder]".to_string());
    input_count += 1;

    // Concatenate all parts
    filter.push_str(&format!(
        "{concat_parts_join}concat=n={input_count}:v=0:a=1[final_audio]",
        concat_parts_join = concat_parts.join("")
    ));

    // Build the output command
    cmd.filter_complex(&filter)
        .output_options([
            "-map",
            "0:v", // Take video from the original
            "-map",
            "[final_audio]", // Use our processed audio
            "-c:v",
            "copy", // Copy video codec (no re-encoding)
            "-c:a",
            &options.codec, // Use specified audio codec
            "-b:a",
            &options.bitrate, // Use specified audio bitrate
            "-ar",
            &options.sample_rate.to_string(), // Use specified sample rate
        ])
        .set_output(output)
        .overwrite(true);

    cmd.execute().map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replacement_options_defaults() {
        let options = ReplacementOptions::default();
        assert_eq!(options.codec, common::DEFAULT_AUDIO_CODEC);
        assert_eq!(options.bitrate, common::DEFAULT_AUDIO_BITRATE);
        assert_eq!(options.sample_rate, common::DEFAULT_SAMPLE_RATE);
        assert_eq!(options.offset, 0.0);
        assert_eq!(options.volume, 1.0);
        assert_eq!(options.original_volume, 0.0);
        assert!(options.match_duration);
        assert!(!options.loop_audio);
        assert_eq!(options.channels, 2);
        assert!(options.stream_index.is_none());
        assert!(!options.normalize_audio);
        assert_eq!(options.audio_volume, 1.0);
        assert!(options.video_start.is_none());
        assert!(options.audio_start.is_none());
        assert!(options.duration.is_none());
    }

    #[test]
    fn test_replacement_options_fluent_api() {
        let options = ReplacementOptions::new()
            .codec("mp3")
            .bitrate("320k")
            .sample_rate(48000)
            .offset(5.0)
            .volume(0.8)
            .original_volume(0.2)
            .match_duration(false)
            .loop_audio(true)
            .channels(1)
            .stream_index(Some(0))
            .normalize_audio(true)
            .audio_volume(0.5)
            .video_start(Some(10.0))
            .audio_start(Some(20.0))
            .duration(Some(30.0));

        assert_eq!(options.codec, "mp3");
        assert_eq!(options.bitrate, "320k");
        assert_eq!(options.sample_rate, 48000);
        assert_eq!(options.offset, 5.0);
        assert_eq!(options.volume, 0.8);
        assert_eq!(options.original_volume, 0.2);
        assert!(!options.match_duration);
        assert!(options.loop_audio);
        assert_eq!(options.channels, 1);
        assert_eq!(options.stream_index, Some(0));
        assert!(options.normalize_audio);
        assert_eq!(options.audio_volume, 0.5);
        assert_eq!(options.video_start, Some(10.0));
        assert_eq!(options.audio_start, Some(20.0));
        assert_eq!(options.duration, Some(30.0));
    }

    #[test]
    fn test_replacement_options_validation() {
        // Test volume normalization
        let options = ReplacementOptions::new().volume(15.0);
        assert_eq!(options.volume, 10.0); // Should be clamped to 10.0

        let options = ReplacementOptions::new().original_volume(15.0);
        assert_eq!(options.original_volume, 10.0); // Should be clamped to 10.0
    }
}
