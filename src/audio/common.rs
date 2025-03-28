/// Common audio utilities and constants.
///
/// This module provides shared constants and utility functions used
/// throughout the audio processing modules.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Default audio codec used for operations
pub const DEFAULT_AUDIO_CODEC: &str = "aac";

/// Default audio bitrate used for operations
pub const DEFAULT_AUDIO_BITRATE: &str = "192k";

/// Default audio sample rate used for operations (in Hz)
pub const DEFAULT_AUDIO_SAMPLE_RATE: u32 = 44100;

/// Maximum volume multiplier allowed (to prevent audio clipping)
pub const MAX_VOLUME_MULTIPLIER: f64 = 10.0;

/// Minimum volume multiplier allowed
pub const MIN_VOLUME_MULTIPLIER: f64 = 0.0;

/// Returns a static reference to a HashSet of supported audio formats.
fn supported_audio_formats() -> &'static HashSet<String> {
    static FORMATS: OnceLock<HashSet<String>> = OnceLock::new();
    FORMATS.get_or_init(|| {
        let formats = [
            "aac", "mp3", "wav", "flac", "ogg", "opus",
            "wma", "m4a", "alac", "ac3", "eac3", "pcm",
        ];
        formats.iter().map(|s| s.to_string()).collect()
    })
}

/// Checks if the given audio format is supported.
///
/// # Arguments
///
/// * `format` - The audio format to check (e.g., "mp3", "aac")
///
/// # Returns
///
/// `true` if the format is supported, `false` otherwise
#[must_use]
pub fn is_supported_audio_format(format: &str) -> bool {
    let format = format.to_lowercase();
    supported_audio_formats().contains(&format)
}

/// Normalizes a volume level to be within acceptable bounds.
///
/// # Arguments
///
/// * `level` - The volume level as a linear multiplier
///
/// # Returns
///
/// The normalized volume level, clamped between MIN_VOLUME_MULTIPLIER and MAX_VOLUME_MULTIPLIER
#[must_use]
pub fn normalize_volume_level(level: f64) -> f64 {
    level.clamp(MIN_VOLUME_MULTIPLIER, MAX_VOLUME_MULTIPLIER)
}

/// Converts a decibel value to a linear multiplier.
///
/// # Arguments
///
/// * `db` - The volume level in decibels
///
/// # Returns
///
/// The equivalent linear multiplier
#[must_use]
pub fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

/// Converts a linear multiplier to a decibel value.
///
/// # Arguments
///
/// * `linear` - The linear multiplier
///
/// # Returns
///
/// The equivalent volume level in decibels
#[must_use]
pub fn linear_to_db(linear: f64) -> f64 {
    if linear <= 0.0 {
        -100.0 // Effectively silent
    } else {
        20.0 * linear.log10()
    }
}

/// Calculates the RMS (Root Mean Square) of an array of audio samples.
///
/// # Arguments
///
/// * `samples` - Array of audio samples
///
/// # Returns
///
/// The RMS value of the samples
#[must_use]
pub fn calculate_rms(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    
    let sum_of_squares = samples.iter().map(|&s| s * s).sum::<f64>();
    (sum_of_squares / samples.len() as f64).sqrt()
}

/// Formats a duration in seconds to a string in HH:MM:SS.mmm format.
///
/// # Arguments
///
/// * `seconds` - The duration in seconds
///
/// # Returns
///
/// A formatted time string
#[must_use]
pub fn format_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds - (hours as f64 * 3600.0)) / 60.0).floor() as u32;
    let secs = seconds - (hours as f64 * 3600.0) - (minutes as f64 * 60.0);
    
    format!("{:02}:{:02}:{:06.3}", hours, minutes, secs)
}

/// Parses a time string in various formats to seconds.
///
/// Supported formats:
/// - HH:MM:SS.mmm
/// - MM:SS.mmm
/// - SS.mmm
///
/// # Arguments
///
/// * `time_str` - The time string to parse
///
/// # Returns
///
/// The time in seconds or None if parsing fails
pub fn parse_time(time_str: &str) -> Option<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    
    match parts.len() {
        // Format: SS.mmm
        1 => parts[0].parse::<f64>().ok(),
        
        // Format: MM:SS.mmm
        2 => {
            let minutes = parts[0].parse::<f64>().ok()?;
            let seconds = parts[1].parse::<f64>().ok()?;
            Some(minutes * 60.0 + seconds)
        },
        
        // Format: HH:MM:SS.mmm
        3 => {
            let hours = parts[0].parse::<f64>().ok()?;
            let minutes = parts[1].parse::<f64>().ok()?;
            let seconds = parts[2].parse::<f64>().ok()?;
            Some(hours * 3600.0 + minutes * 60.0 + seconds)
        },
        
        // Unknown format
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supported_audio_formats() {
        assert!(is_supported_audio_format("mp3"));
        assert!(is_supported_audio_format("aac"));
        assert!(is_supported_audio_format("flac"));
        assert!(is_supported_audio_format("MP3")); // Case insensitive
        assert!(!is_supported_audio_format("xyz"));
    }
    
    #[test]
    fn test_volume_normalization() {
        assert_eq!(normalize_volume_level(0.5), 0.5);
        assert_eq!(normalize_volume_level(15.0), MAX_VOLUME_MULTIPLIER);
        assert_eq!(normalize_volume_level(-1.0), MIN_VOLUME_MULTIPLIER);
    }
    
    #[test]
    fn test_db_to_linear_conversion() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-10);
        assert!((db_to_linear(6.0) - 2.0).abs() < 1e-10);
        assert!((db_to_linear(-6.0) - 0.5).abs() < 1e-10);
    }
    
    #[test]
    fn test_linear_to_db_conversion() {
        assert!((linear_to_db(1.0) - 0.0).abs() < 1e-10);
        assert!((linear_to_db(2.0) - 6.0).abs() < 0.1);
        assert!((linear_to_db(0.5) + 6.0).abs() < 0.1);
        assert_eq!(linear_to_db(0.0), -100.0);
    }
    
    #[test]
    fn test_calculate_rms() {
        let samples = vec![0.0, 0.0, 0.0];
        assert_eq!(calculate_rms(&samples), 0.0);
        
        let samples = vec![1.0, 1.0, 1.0, 1.0];
        assert_eq!(calculate_rms(&samples), 1.0);
        
        let samples = vec![2.0, 2.0, 2.0, 2.0];
        assert_eq!(calculate_rms(&samples), 2.0);
        
        let samples = vec![-3.0, 3.0, -3.0, 3.0];
        assert_eq!(calculate_rms(&samples), 3.0);
    }
    
    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0.0), "00:00:00.000");
        assert_eq!(format_time(1.5), "00:00:01.500");
        assert_eq!(format_time(61.25), "00:01:01.250");
        assert_eq!(format_time(3661.75), "01:01:01.750");
    }
    
    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("1.5"), Some(1.5));
        assert_eq!(parse_time("01:30"), Some(90.0));
        assert_eq!(parse_time("01:01:30"), Some(3690.0));
        assert_eq!(parse_time("invalid"), None);
    }
} 