/// Audio volume adjustment functionality.
///
/// This module provides functions for adjusting the volume of audio tracks in video files.
/// It supports both global volume adjustment for the entire file and temporal volume
/// adjustments for specific segments.
use std::path::Path;

use crate::audio::common;
use crate::audio::error::{Error, Result};
use crate::ffmpeg::{FFmpeg, command::FFmpegCommand};

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
    ///
    /// # Returns
    ///
    /// The volume adjustment as a linear multiplier
    #[must_use]
    pub fn as_linear(&self) -> f64 {
        match *self {
            Self::Linear(value) => value,
            Self::Decibel(db) => common::db_to_linear(db),
        }
    }

    /// Converts the adjustment to decibels.
    ///
    /// # Returns
    ///
    /// The volume adjustment in decibels
    #[must_use]
    pub fn as_db(&self) -> f64 {
        match *self {
            Self::Linear(value) => common::linear_to_db(value),
            Self::Decibel(db) => db,
        }
    }

    /// Creates a new linear volume adjustment.
    ///
    /// # Arguments
    ///
    /// * `value` - Linear multiplier (e.g., 1.5 = 150% volume)
    ///
    /// # Returns
    ///
    /// A new VolumeAdjustment::Linear
    #[must_use]
    pub fn linear(value: f64) -> Self {
        Self::Linear(common::normalize_volume_level(value))
    }

    /// Creates a new decibel volume adjustment.
    ///
    /// # Arguments
    ///
    /// * `db` - Decibel adjustment (e.g., 6.0 = +6dB)
    ///
    /// # Returns
    ///
    /// A new VolumeAdjustment::Decibel
    #[must_use]
    pub fn decibel(db: f64) -> Self {
        Self::Decibel(db.clamp(-60.0, 20.0))
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
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `adjustment` - The volume adjustment to apply
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
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
    if adjustment.as_linear() < 0.0 {
        return Err(Error::InvalidVolumeLevel(adjustment.as_linear()));
    }

    let volume_expr = match adjustment {
        VolumeAdjustment::Linear(value) => format!("volume={value}"),
        VolumeAdjustment::Decibel(db) => format!("volume={db}dB"),
    };

    let mut cmd = FFmpegCommand::new(ffmpeg);
    cmd.input(input)
        .output(output)
        .output_options(["-filter:a", &volume_expr, "-c:v", "copy"])
        .overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Adjusts the volume of audio within specific time ranges.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `adjustments` - List of temporal volume adjustments to apply
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
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
    if adjustments.is_empty() {
        return Err(Error::ProcessingError(
            "No volume adjustments specified".to_string(),
        ));
    }

    // Build a complex filter string that applies volume adjustments to specific segments
    let mut filter_parts = Vec::new();

    for adj in adjustments.iter() {
        if adj.adjustment.as_linear() < 0.0 {
            return Err(Error::InvalidVolumeLevel(adj.adjustment.as_linear()));
        }

        let volume_expr = match adj.adjustment {
            VolumeAdjustment::Linear(value) => format!("volume={value}"),
            VolumeAdjustment::Decibel(db) => format!("volume={db}dB"),
        };

        let time_expr = match adj.end_time {
            Some(end) if end > adj.start_time => {
                let start = adj.start_time;
                format!("between(t,{start:.3},{end:.3})")
            }
            Some(_) => {
                return Err(Error::ProcessingError(
                    "End time must be greater than start time".to_string(),
                ));
            }
            None => {
                let start = adj.start_time;
                format!("gte(t,{start:.3})")
            }
        };

        filter_parts.push(format!("{volume_expr}:when='{time_expr}'"));
    }

    let volume_filter = format!("volume={}", filter_parts.join(","));

    let mut cmd = FFmpegCommand::new(ffmpeg);
    cmd.input(input)
        .output(output)
        .output_options(["-filter:a", &volume_filter, "-c:v", "copy"])
        .overwrite(true);

    cmd.execute().map_err(Error::from)
}

/// Normalizes audio volume to a target level.
///
/// # Arguments
///
/// * `ffmpeg` - The FFmpeg instance to use
/// * `input` - Path to the input file
/// * `output` - Path to the output file
/// * `target_level` - Target audio level in dB (typically -18 to -14)
///
/// # Returns
///
/// A Result indicating success or an error
///
/// # Errors
///
/// Returns an error if the file can't be processed or doesn't contain audio
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
    // Validate target level is in a reasonable range
    if !(-70.0..=0.0).contains(&target_level) {
        return Err(Error::ProcessingError(format!(
            "Target level {target_level} dB is outside reasonable range (-70 to 0 dB)"
        )));
    }

    // Two-pass normalization:
    // 1. First analyze audio to find the current levels
    // 2. Then apply normalization based on the analysis

    let mut cmd = FFmpegCommand::new(ffmpeg);
    cmd.input(input.as_ref())
        .output(output)
        .output_options([
            "-filter:a",
            &format!("loudnorm=I={target_level}:TP=-1.5:LRA=11:print_format=json"),
            "-c:v",
            "copy",
        ])
        .overwrite(true);

    cmd.execute().map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_adjustment_linear() {
        let adj = VolumeAdjustment::linear(1.5);
        assert_eq!(adj.as_linear(), 1.5);
        assert!((adj.as_db() - 3.52).abs() < 0.1); // ~3.52 dB
    }

    #[test]
    fn test_volume_adjustment_decibel() {
        let adj = VolumeAdjustment::decibel(6.0);
        assert_eq!(adj.as_db(), 6.0);
        assert!((adj.as_linear() - 2.0).abs() < 0.01); // ~2.0
    }

    #[test]
    fn test_volume_adjustment_clamp() {
        let adj = VolumeAdjustment::linear(15.0);
        assert_eq!(adj.as_linear(), 10.0); // Clamped to 10.0

        let adj = VolumeAdjustment::decibel(30.0);
        assert_eq!(adj.as_db(), 20.0); // Clamped to 20.0
    }
}
