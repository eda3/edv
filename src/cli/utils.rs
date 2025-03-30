#[allow(dead_code)]
/// Miscellaneous utility functions for the CLI.
///
/// This module provides various utility functions that are used across
/// different parts of the CLI application. These utilities handle common tasks
/// that don't fit into other more specific modules.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use super::{Error, Result};

/// Represents a time position in a media file, with multiple possible formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimePosition {
    /// The time position in seconds.
    seconds: f64,
}

impl TimePosition {
    /// Creates a new time position from seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The time position in seconds
    ///
    /// # Returns
    ///
    /// A new `TimePosition` instance.
    #[must_use]
    pub fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }

    /// Creates a new time position from hours, minutes, and seconds.
    ///
    /// # Arguments
    ///
    /// * `hours` - The hours component
    /// * `minutes` - The minutes component
    /// * `seconds` - The seconds component
    ///
    /// # Returns
    ///
    /// A new `TimePosition` instance.
    #[must_use]
    pub fn from_hms(hours: u32, minutes: u32, seconds: f64) -> Self {
        let total_seconds = (hours as f64) * 3600.0 + (minutes as f64) * 60.0 + seconds;
        Self {
            seconds: total_seconds,
        }
    }

    /// Gets the time position in seconds.
    ///
    /// # Returns
    ///
    /// The time position in seconds.
    #[must_use]
    #[allow(dead_code)]
    pub fn as_seconds(&self) -> f64 {
        self.seconds
    }

    /// Gets the time position as a Duration.
    ///
    /// # Returns
    ///
    /// The time position as a Duration.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    #[allow(dead_code)]
    pub fn as_duration(&self) -> Duration {
        let secs = (self.seconds.floor().max(0.0)) as u64;
        let nanos = ((self.seconds - self.seconds.floor()).max(0.0) * 1_000_000_000.0) as u32;
        Duration::new(secs, nanos)
    }

    /// Formats the time position as a string in the format "HH:MM:SS.mmm".
    ///
    /// # Returns
    ///
    /// A string representation of the time position.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    #[allow(dead_code)]
    pub fn format(&self) -> String {
        let total_seconds = self.seconds;
        let hours = ((total_seconds / 3600.0).floor().max(0.0)) as u32;
        let minutes = (((total_seconds % 3600.0) / 60.0).floor().max(0.0)) as u32;
        let seconds = total_seconds % 60.0;

        format!("{hours:02}:{minutes:02}:{seconds:06.3}")
    }

    /// Formats the time position in a format suitable for FFmpeg.
    ///
    /// # Returns
    ///
    /// A string representation of the time position suitable for FFmpeg.
    #[must_use]
    #[allow(dead_code)]
    pub fn format_for_ffmpeg(&self) -> String {
        self.format()
    }
}

impl FromStr for TimePosition {
    type Err = Error;

    /// Parses a string into a TimePosition.
    ///
    /// Supports the following formats:
    /// - Seconds (e.g., "123.45")
    /// - Minutes:Seconds (e.g., "12:34.56")
    /// - Hours:Minutes:Seconds (e.g., "1:23:45.678")
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `TimePosition` or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed as a valid time position.
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();

        match parts.len() {
            // Just seconds
            1 => {
                let seconds = parts[0]
                    .parse::<f64>()
                    .map_err(|_| Error::InvalidTimeFormat(s.to_string()))?;
                Ok(Self::from_seconds(seconds))
            }
            // Minutes:Seconds
            2 => {
                let minutes = parts[0]
                    .parse::<u32>()
                    .map_err(|_| Error::InvalidTimeFormat(s.to_string()))?;
                let seconds = parts[1]
                    .parse::<f64>()
                    .map_err(|_| Error::InvalidTimeFormat(s.to_string()))?;
                Ok(Self::from_hms(0, minutes, seconds))
            }
            // Hours:Minutes:Seconds
            3 => {
                let hours = parts[0]
                    .parse::<u32>()
                    .map_err(|_| Error::InvalidTimeFormat(s.to_string()))?;
                let minutes = parts[1]
                    .parse::<u32>()
                    .map_err(|_| Error::InvalidTimeFormat(s.to_string()))?;
                let seconds = parts[2]
                    .parse::<f64>()
                    .map_err(|_| Error::InvalidTimeFormat(s.to_string()))?;
                Ok(Self::from_hms(hours, minutes, seconds))
            }
            _ => Err(Error::InvalidTimeFormat(s.to_string())),
        }
    }
}

/// Checks if a file exists and has the expected extension.
///
/// # Arguments
///
/// * `path` - The path to check
/// * `expected_extension` - The expected file extension (without the dot)
///
/// # Returns
///
/// `true` if the file exists and has the expected extension, `false` otherwise.
#[must_use]
#[allow(dead_code)]
pub fn file_has_extension(path: &Path, expected_extension: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(expected_extension))
        .unwrap_or(false)
}

/// Gets all media files in a directory with the specified extensions.
///
/// # Arguments
///
/// * `dir` - The directory to search
/// * `extensions` - The file extensions to include (without the dot)
///
/// # Returns
///
/// A vector of paths to media files.
///
/// # Errors
///
/// Returns an IO error if the directory cannot be read.
#[allow(dead_code)]
pub fn get_media_files(dir: &Path, extensions: &[&str]) -> io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir)?;

    Ok(entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|ext| {
                    extensions
                        .iter()
                        .any(|&valid_ext| ext.eq_ignore_ascii_case(valid_ext))
                })
        })
        .collect())
}

/// Creates a directory if it doesn't exist.
///
/// # Arguments
///
/// * `path` - The directory path to create
///
/// # Returns
///
/// The path that was created or already existed.
///
/// # Errors
///
/// Returns an IO error if the directory cannot be created.
#[allow(dead_code)]
pub fn ensure_dir_exists(path: &Path) -> io::Result<&Path> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    } else if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Path exists but is not a directory: {}", path.display()),
        ));
    }

    Ok(path)
}

/// Ensures a file name has the specified extension.
///
/// If the file already has the extension, it is returned unchanged.
/// Otherwise, the extension is added.
///
/// # Arguments
///
/// * `file_name` - The file name to check
/// * `extension` - The extension to ensure (without the dot)
///
/// # Returns
///
/// A string with the ensured extension.
#[must_use]
#[allow(dead_code)]
pub fn ensure_extension(file_name: &str, extension: &str) -> String {
    if file_name
        .to_lowercase()
        .ends_with(&format!(".{extension}").to_lowercase())
    {
        file_name.to_string()
    } else {
        format!("{file_name}.{extension}")
    }
}

/// Generates a temporary file path with a unique name.
///
/// # Arguments
///
/// * `prefix` - A prefix for the filename
/// * `extension` - The file extension (without the dot)
///
/// # Returns
///
/// A path to a temporary file that doesn't exist yet.
///
/// # Errors
///
/// Returns an IO error if a temporary directory cannot be accessed.
#[allow(dead_code)]
pub fn temp_file_path(prefix: &str, extension: &str) -> io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let temp_path = temp_dir.join(format!("{prefix}_{timestamp}.{extension}"));

    // If the file already exists (very unlikely), try again with a different name
    if temp_path.exists() {
        let random_suffix = rand::random::<u16>();
        Ok(temp_dir.join(format!("{prefix}_{timestamp}_{random_suffix}.{extension}")))
    } else {
        Ok(temp_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::EPSILON;

    #[test]
    fn test_time_position_from_seconds() {
        let pos = TimePosition::from_seconds(123.45);
        assert!((pos.as_seconds() - 123.45).abs() < EPSILON);
    }

    #[test]
    fn test_time_position_from_hms() {
        let pos = TimePosition::from_hms(1, 2, 3.5);
        // 1h 2m 3.5s = 3600 + 120 + 3.5 = 3723.5s
        assert!((pos.as_seconds() - 3723.5).abs() < EPSILON);
    }

    #[test]
    fn test_time_position_format() {
        let pos = TimePosition::from_hms(1, 2, 3.5);
        assert_eq!(pos.format(), "01:02:03.500");

        let pos = TimePosition::from_seconds(123.456);
        assert_eq!(pos.format(), "00:02:03.456");
    }

    #[test]
    fn test_time_position_from_str() {
        // Seconds only
        let pos: TimePosition = "123.45".parse().unwrap();
        assert!((pos.as_seconds() - 123.45).abs() < EPSILON);

        // Minutes:Seconds
        let pos: TimePosition = "10:20.5".parse().unwrap();
        assert!((pos.as_seconds() - 620.5).abs() < EPSILON);

        // Hours:Minutes:Seconds
        let pos: TimePosition = "1:02:03.5".parse().unwrap();
        assert!((pos.as_seconds() - 3723.5).abs() < EPSILON);

        // Invalid format
        let result: Result<TimePosition> = "invalid".parse();
        assert!(result.is_err());

        let result: Result<TimePosition> = "1:2:3:4".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_extension() {
        assert_eq!(ensure_extension("file", "mp4"), "file.mp4");
        assert_eq!(ensure_extension("file.mp4", "mp4"), "file.mp4");
        assert_eq!(ensure_extension("file.MP4", "mp4"), "file.MP4");
        assert_eq!(ensure_extension("file.txt", "mp4"), "file.txt.mp4");
    }

    #[test]
    fn test_file_has_extension() {
        // Note: These tests don't actually check files on disk
        let path = PathBuf::from("file.mp4");
        assert!(file_has_extension(&path, "mp4"));
        assert!(file_has_extension(&path, "MP4"));
        assert!(!file_has_extension(&path, "txt"));

        let path = PathBuf::from("file");
        assert!(!file_has_extension(&path, "mp4"));
    }
}
