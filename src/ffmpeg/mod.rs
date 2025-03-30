use serde_json;
/// `FFmpeg` integration for the edv video editor.
///
/// This module provides functionality for detecting, validating, and
/// interacting with `FFmpeg`. It serves as the core abstraction layer between
/// the application and `FFmpeg`, handling command construction, execution, and
/// result parsing.
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::{env, io};
use thiserror::Error;

// Submodules
pub mod command;
pub mod error;

/// Errors that can occur in the `FFmpeg` module.
#[derive(Error, Debug)]
pub enum Error {
    /// `FFmpeg` executable not found.
    #[error("FFmpeg executable not found")]
    NotFound,

    /// `FFmpeg` executable path is not valid.
    #[error("FFmpeg path is not valid: {0}")]
    InvalidPath(String),

    /// `FFmpeg` version is not supported.
    #[error("FFmpeg version {actual} is not supported (minimum: {required})")]
    UnsupportedVersion {
        /// The actual `FFmpeg` version detected.
        actual: Version,
        /// The minimum required `FFmpeg` version.
        required: Version,
    },

    /// Error executing `FFmpeg` command.
    #[error("Error executing FFmpeg command: {0}")]
    ExecutionError(String),

    /// Error parsing `FFmpeg` output.
    #[error("Error parsing FFmpeg output: {0}")]
    OutputParseError(String),

    /// IO error occurred.
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// `FFmpeg` process terminated with non-zero exit code.
    #[error("FFmpeg process terminated: {message}")]
    ProcessTerminated {
        /// The exit code of the process, if available.
        exit_code: Option<i32>,
        /// The error message.
        message: String,
    },

    /// Invalid time format provided.
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),

    /// Missing required argument.
    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    /// Invalid argument provided.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Result type for `FFmpeg` operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an `FFmpeg` version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Patch version number.
    pub patch: u32,
}

impl Version {
    /// Creates a new version from components.
    ///
    /// # Arguments
    ///
    /// * `major` - The major version number
    /// * `minor` - The minor version number
    /// * `patch` - The patch version number
    ///
    /// # Returns
    ///
    /// A new `Version` instance.
    #[must_use]
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = Error;

    /// Parses a version string into a `Version` object.
    ///
    /// # Arguments
    ///
    /// * `s` - The version string in format "major.minor.patch"
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `Version` or an error.
    ///
    /// # Errors
    ///
    /// Returns a `ParseError` if the version string is invalid.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 {
            return Err(Error::OutputParseError(format!(
                "Invalid version format: {s}, expected major.minor.patch"
            )));
        }

        let parse_component = |idx: usize, name: &str| {
            parts[idx].parse::<u32>().map_err(|_| {
                Error::OutputParseError(format!("Invalid {name} version component: {}", parts[idx]))
            })
        };

        let major = parse_component(0, "major")?;
        let minor = parse_component(1, "minor")?;
        let patch = parse_component(2, "patch")?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

/// Represents a detected `FFmpeg` installation.
#[derive(Debug, Clone)]
pub struct FFmpeg {
    /// The path to the `FFmpeg` executable.
    path: PathBuf,
    /// The `FFmpeg` version.
    version: Version,
}

impl FFmpeg {
    /// The minimum supported `FFmpeg` version.
    pub const MIN_VERSION: Version = Version {
        major: 4,
        minor: 0,
        patch: 0,
    };

    /// Creates a new `FFmpeg` instance.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the `FFmpeg` executable
    /// * `version` - The detected `FFmpeg` version
    ///
    /// # Returns
    ///
    /// A new `FFmpeg` instance.
    #[must_use]
    pub fn new(path: PathBuf, version: Version) -> Self {
        Self { path, version }
    }

    /// Gets the path to the `FFmpeg` executable.
    ///
    /// # Returns
    ///
    /// A reference to the path of the `FFmpeg` executable.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets the `FFmpeg` version.
    ///
    /// # Returns
    ///
    /// A reference to the `FFmpeg` version.
    #[must_use]
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Detects the `FFmpeg` installation.
    ///
    /// Searches for `FFmpeg` in the system PATH and validates it.
    ///
    /// # Returns
    ///
    /// A Result containing the `FFmpeg` installation if found and valid.
    ///
    /// # Errors
    ///
    /// Returns an error if `FFmpeg` is not found or not compatible.
    pub fn detect() -> Result<Self> {
        // First try to find in PATH
        if let Ok(ffmpeg) = Self::detect_in_path() {
            return Ok(ffmpeg);
        }

        // Then try common installation directories
        if let Ok(ffmpeg) = Self::detect_in_common_locations() {
            return Ok(ffmpeg);
        }

        Err(Error::NotFound)
    }

    /// Detects `FFmpeg` installation from a specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check for `FFmpeg` executable
    ///
    /// # Returns
    ///
    /// A Result containing the `FFmpeg` installation if found and valid.
    ///
    /// # Errors
    ///
    /// Returns an error if `FFmpeg` is not found at the path or not compatible.
    pub fn detect_at_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(Error::NotFound);
        }

        // Try to get version
        let version = Self::parse_version_from_command(&path)?;

        // Check version compatibility
        if version < Self::MIN_VERSION {
            return Err(Error::UnsupportedVersion {
                actual: version,
                required: Self::MIN_VERSION,
            });
        }

        Ok(Self::new(path, version))
    }

    /// Detects `FFmpeg` in the system PATH.
    ///
    /// # Returns
    ///
    /// A Result containing the `FFmpeg` installation if found and valid.
    ///
    /// # Errors
    ///
    /// Returns an error if `FFmpeg` is not found in PATH or not compatible.
    fn detect_in_path() -> Result<Self> {
        // Try to find ffmpeg in PATH
        let ffmpeg_name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };

        if let Ok(path) = which::which(ffmpeg_name) {
            return Self::detect_at_path(path);
        }

        Err(Error::NotFound)
    }

    /// Detects `FFmpeg` in common installation locations.
    ///
    /// # Returns
    ///
    /// A Result containing the `FFmpeg` installation if found and valid.
    ///
    /// # Errors
    ///
    /// Returns an error if `FFmpeg` is not found in common locations or not compatible.
    fn detect_in_common_locations() -> Result<Self> {
        let common_locations = Self::get_common_locations();

        for location in common_locations {
            if location.exists() {
                match Self::detect_at_path(&location) {
                    Ok(ffmpeg) => return Ok(ffmpeg),
                    Err(_) => continue,
                }
            }
        }

        Err(Error::NotFound)
    }

    /// Gets common installation locations for `FFmpeg`.
    ///
    /// # Returns
    ///
    /// A vector of paths to check for `FFmpeg`.
    #[must_use]
    fn get_common_locations() -> Vec<PathBuf> {
        let mut locations = Vec::new();

        if cfg!(windows) {
            // Windows common locations
            if let Some(program_files) = env::var_os("ProgramFiles") {
                locations.push(
                    PathBuf::from(program_files)
                        .join("FFmpeg")
                        .join("bin")
                        .join("ffmpeg.exe"),
                );
            }
            if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
                locations.push(
                    PathBuf::from(program_files_x86)
                        .join("FFmpeg")
                        .join("bin")
                        .join("ffmpeg.exe"),
                );
            }
            locations.push(PathBuf::from(r"C:\FFmpeg\bin\ffmpeg.exe"));
        } else if cfg!(target_os = "macos") {
            // macOS common locations
            locations.push(PathBuf::from("/usr/local/bin/ffmpeg"));
            locations.push(PathBuf::from("/opt/homebrew/bin/ffmpeg"));
            locations.push(PathBuf::from("/opt/local/bin/ffmpeg"));
        } else {
            // Linux/Unix common locations
            locations.push(PathBuf::from("/usr/bin/ffmpeg"));
            locations.push(PathBuf::from("/usr/local/bin/ffmpeg"));
            locations.push(PathBuf::from("/opt/ffmpeg/bin/ffmpeg"));
        }

        locations
    }

    /// Parses the `FFmpeg` version from command output.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the `FFmpeg` executable
    ///
    /// # Returns
    ///
    /// A Result containing the parsed version.
    ///
    /// # Errors
    ///
    /// Returns an error if the version cannot be determined.
    fn parse_version_from_command(path: &Path) -> Result<Version> {
        let output = Command::new(path)
            .arg("-version")
            .output()
            .map_err(|e| Error::ExecutionError(format!("Failed to execute FFmpeg: {e}")))?;

        if !output.status.success() {
            return Err(Error::ExecutionError(format!(
                "FFmpeg returned error code: {}",
                output.status
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        Self::parse_version_from_output(&output_str)
    }

    /// Parses the `FFmpeg` version from version output.
    ///
    /// # Arguments
    ///
    /// * `output` - The output of the ffmpeg -version command
    ///
    /// # Returns
    ///
    /// A Result containing the parsed version.
    ///
    /// # Errors
    ///
    /// Returns an error if the version cannot be parsed.
    fn parse_version_from_output(output: &str) -> Result<Version> {
        // First line should be "ffmpeg version X.Y.Z"
        let first_line = output
            .lines()
            .next()
            .ok_or_else(|| Error::OutputParseError("Empty FFmpeg version output".to_string()))?;

        // Extract version number
        let version_str = first_line
            .split_whitespace()
            .nth(2) // "ffmpeg" "version" "X.Y.Z"
            .ok_or_else(|| {
                Error::OutputParseError(format!("Invalid FFmpeg version line: {first_line}"))
            })?;

        // Parse version
        version_str.parse()
    }

    /// Validates that the `FFmpeg` installation is compatible.
    ///
    /// # Returns
    ///
    /// A Result indicating validation success.
    ///
    /// # Errors
    ///
    /// Returns an error if the validation fails.
    pub fn validate(&self) -> Result<()> {
        if self.version < Self::MIN_VERSION {
            return Err(Error::UnsupportedVersion {
                actual: self.version.clone(),
                required: Self::MIN_VERSION,
            });
        }
        Ok(())
    }

    /// Gets detailed information about a media file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the media file
    ///
    /// # Returns
    ///
    /// A Result containing a `MediaInfo` struct with details about the file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The `FFmpeg` process fails to start or returns a non-zero exit code
    /// * The output cannot be parsed
    /// * The file is not a valid media file
    pub fn get_media_info<P: AsRef<Path>>(&self, file_path: P) -> Result<MediaInfo> {
        let path = file_path.as_ref();

        if !path.exists() {
            return Err(Error::InvalidPath(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Use ffprobe to get media information
        let output = std::process::Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(path)
            .output()
            .map_err(|e| Error::ExecutionError(format!("Failed to execute ffprobe: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::ProcessTerminated {
                exit_code: output.status.code(),
                message: format!("ffprobe process failed: {stderr}"),
            });
        }

        // Parse the JSON output
        let output_str = String::from_utf8_lossy(&output.stdout);
        let media_info: MediaInfo = serde_json::from_str(&output_str)
            .map_err(|e| Error::OutputParseError(format!("Failed to parse ffprobe output: {e}")))?;

        Ok(media_info)
    }
}

/// Represents media format information.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FormatInfo {
    /// The filename.
    pub filename: String,
    /// The number of streams.
    #[serde(default)]
    pub nb_streams: i32,
    /// The number of programs.
    #[serde(default)]
    pub nb_programs: i32,
    /// The format name.
    #[serde(default)]
    pub format_name: String,
    /// The format long name.
    #[serde(rename = "format_long_name", default)]
    pub format_long_name: String,
    /// The start time in seconds.
    #[serde(default)]
    pub start_time: Option<String>,
    /// The duration in seconds.
    #[serde(default)]
    pub duration: Option<String>,
    /// The size in bytes.
    #[serde(default)]
    pub size: Option<String>,
    /// The bit rate in bits per second.
    #[serde(default)]
    pub bit_rate: Option<String>,
    /// The probe score (higher is better).
    #[serde(default)]
    pub probe_score: i32,
    /// Additional tags.
    #[serde(default)]
    pub tags: Option<std::collections::HashMap<String, String>>,
}

/// Represents a media stream (video, audio, subtitle, etc.).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StreamInfo {
    /// The index of the stream.
    pub index: i32,
    /// The codec type (video, audio, subtitle, etc.).
    #[serde(rename = "codec_type")]
    pub codec_type: String,
    /// The codec name.
    #[serde(rename = "codec_name", default)]
    pub codec_name: String,
    /// The codec long name.
    #[serde(rename = "codec_long_name", default)]
    pub codec_long_name: String,
    /// The width (for video streams).
    #[serde(default)]
    pub width: Option<i32>,
    /// The height (for video streams).
    #[serde(default)]
    pub height: Option<i32>,
    /// The pixel format (for video streams).
    #[serde(rename = "pix_fmt", default)]
    pub pixel_format: Option<String>,
    /// The frame rate (for video streams).
    #[serde(rename = "r_frame_rate", default)]
    pub frame_rate: Option<String>,
    /// The sample rate (for audio streams).
    #[serde(rename = "sample_rate", default)]
    pub sample_rate: Option<String>,
    /// The number of channels (for audio streams).
    #[serde(default)]
    pub channels: Option<i32>,
    /// The channel layout (for audio streams).
    #[serde(rename = "channel_layout", default)]
    pub channel_layout: Option<String>,
    /// The bit rate (for audio/video streams).
    #[serde(default)]
    pub bit_rate: Option<String>,
    /// Additional tags.
    #[serde(default)]
    pub tags: Option<std::collections::HashMap<String, String>>,
}

/// Represents comprehensive media information.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MediaInfo {
    /// Information about the format (container).
    pub format: FormatInfo,
    /// Information about the streams (video, audio, subtitle, etc.).
    pub streams: Vec<StreamInfo>,
}

impl MediaInfo {
    /// Gets the video streams.
    ///
    /// # Returns
    ///
    /// A vector of references to video streams.
    #[must_use]
    pub fn video_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|stream| stream.codec_type == "video")
            .collect()
    }

    /// Gets the audio streams.
    ///
    /// # Returns
    ///
    /// A vector of references to audio streams.
    #[must_use]
    pub fn audio_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|stream| stream.codec_type == "audio")
            .collect()
    }

    /// Gets the subtitle streams.
    ///
    /// # Returns
    ///
    /// A vector of references to subtitle streams.
    #[must_use]
    pub fn subtitle_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|stream| stream.codec_type == "subtitle")
            .collect()
    }

    /// Gets the duration in seconds.
    ///
    /// # Returns
    ///
    /// The duration in seconds, or None if not available.
    #[must_use]
    pub fn duration_seconds(&self) -> Option<f64> {
        self.format
            .duration
            .as_ref()
            .and_then(|s| s.parse::<f64>().ok())
    }

    /// Gets the bit rate in bits per second.
    ///
    /// # Returns
    ///
    /// The bit rate in bits per second, or None if not available.
    #[must_use]
    pub fn bit_rate(&self) -> Option<u64> {
        self.format
            .bit_rate
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!("4.3.2".parse::<Version>().unwrap(), Version::new(4, 3, 2));

        assert_eq!("5.0.0".parse::<Version>().unwrap(), Version::new(5, 0, 0));

        // Test invalid formats
        assert!("4.3".parse::<Version>().is_err());
        assert!("invalid".parse::<Version>().is_err());
        assert!("4.a.2".parse::<Version>().is_err());
    }

    #[test]
    fn test_version_display() {
        let version = Version::new(4, 3, 2);
        assert_eq!(version.to_string(), "4.3.2");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(4, 3, 2);
        let v2 = Version::new(5, 0, 0);
        let v3 = Version::new(4, 3, 2);
        let v4 = Version::new(4, 3, 1);

        assert!(v1 < v2);
        assert!(v2 > v1);
        assert_eq!(v1, v3);
        assert!(v1 > v4);
    }

    #[test]
    fn test_parse_version_from_output() {
        // Test with a typical FFmpeg version output
        let output = "ffmpeg version 4.3.2 Copyright (c) 2000-2021 the FFmpeg developers";
        let version = FFmpeg::parse_version_from_output(output).unwrap();
        assert_eq!(version, Version::new(4, 3, 2));

        // Test with a different format
        let output = "ffmpeg version n4.4 Copyright (c) 2000-2021 the FFmpeg developers";
        assert!(FFmpeg::parse_version_from_output(output).is_err());

        // Test with invalid output
        let output = "not a valid ffmpeg output";
        assert!(FFmpeg::parse_version_from_output(output).is_err());
    }
}
