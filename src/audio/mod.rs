/// Audio processing module for the edv video editor.
///
/// This module provides functionality for working with audio in video files,
/// including volume adjustment, audio extraction, audio replacement, and
/// various audio effects.
///
/// # Examples
///
/// ```rust,no_run // Add no_run to prevent running during tests if ffmpeg isn't present
/// use edv::ffmpeg::FFmpeg;
/// use edv::audio::volume::{adjust_volume, VolumeAdjustment};
/// use std::path::Path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> { // Add main fn for compilability
/// // Assuming ffmpeg is in PATH
/// let ffmpeg = FFmpeg::detect()?; // Use detect() instead of locate()
/// adjust_volume(&ffmpeg, Path::new("input.mp4"), Path::new("output.mp4"), VolumeAdjustment::Decibel(-6.0))?;
/// # Ok(())
/// # }
/// ```
// Re-export public types from submodules
pub use self::error::Error;
pub use self::error::Result;

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

    /// Validates if the given audio format is supported.
    ///
    /// # Arguments
    ///
    /// * `format` - The audio format to check
    ///
    /// # Returns
    ///
    /// `true` if the format is supported, `false` otherwise
    #[must_use]
    pub fn is_supported_format(format: &str) -> bool {
        SUPPORTED_AUDIO_FORMATS.contains(&format.to_lowercase().as_str())
    }

    /// Validates and normalizes an audio level value.
    ///
    /// # Arguments
    ///
    /// * `level` - The volume level (will be clamped to 0.0 - 10.0)
    ///
    /// # Returns
    ///
    /// The normalized volume level
    #[must_use]
    pub fn normalize_volume_level(level: f64) -> f64 {
        level.clamp(0.0, 10.0)
    }

    /// Converts dB value to a linear multiplier.
    ///
    /// # Arguments
    ///
    /// * `db` - The decibel value
    ///
    /// # Returns
    ///
    /// The linear multiplier equivalent
    #[must_use]
    pub fn db_to_linear(db: f64) -> f64 {
        10.0_f64.powf(db / 20.0)
    }

    /// Converts linear multiplier to dB value.
    ///
    /// # Arguments
    ///
    /// * `linear` - The linear multiplier
    ///
    /// # Returns
    ///
    /// The decibel equivalent
    #[must_use]
    pub fn linear_to_db(linear: f64) -> f64 {
        20.0 * linear.max(1e-10).log10()
    }
}

/// Wraps `std::result::Result` with `audio::Error` as the default error type.
///
/// # Examples
///
/// ```rust,no_run // Add no_run
/// use edv::ffmpeg::FFmpeg;
/// use edv::audio::volume::{adjust_volume, VolumeAdjustment};
/// use edv::audio::Result;
/// use std::path::Path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> { // Add main fn
/// fn process_audio() -> Result<()> {
///     let ffmpeg = FFmpeg::detect()?; // Use detect()
///     adjust_volume(
///         &ffmpeg,
///         Path::new("input.mp4"),
///         Path::new("output.mp4"),
///         VolumeAdjustment::Linear(0.5) // Use VolumeAdjustment variant
///     )?;
///     Ok(())
/// }
///
/// if let Err(e) = process_audio() {
///     eprintln!("Audio processing failed: {}", e);
/// }
/// # Ok(())
/// # }
/// ```
// pub type Result<T> = std::result::Result<T, Error>; // REMOVED: Conflicts with re-export

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_format() {
        assert!(common::is_supported_format("mp3"));
        assert!(common::is_supported_format("MP3"));
        assert!(common::is_supported_format("wav"));
        assert!(!common::is_supported_format("xyz"));
    }

    #[test]
    fn test_normalize_volume_level() {
        assert_eq!(common::normalize_volume_level(0.5), 0.5);
        assert_eq!(common::normalize_volume_level(-1.0), 0.0);
        assert_eq!(common::normalize_volume_level(15.0), 10.0);
    }

    #[test]
    fn test_db_to_linear() {
        assert!((common::db_to_linear(0.0) - 1.0).abs() < 1e-10);
        assert!((common::db_to_linear(6.0) - 1.9952623149688797).abs() < 1e-10);
        assert!((common::db_to_linear(-6.0) - 0.501187233627272).abs() < 1e-10);
    }

    #[test]
    fn test_linear_to_db() {
        assert!((common::linear_to_db(1.0) - 0.0).abs() < 1e-10);
        assert!((common::linear_to_db(2.0) - 6.020599913279624).abs() < 1e-10);
        assert!((common::linear_to_db(0.5) - (-6.020599913279624)).abs() < 1e-10);
    }
}
