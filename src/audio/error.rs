/// Error types for the audio processing module.
///
/// This module defines error types specific to audio processing operations.
/// It uses `thiserror` for ergonomic error definitions and handling.
use std::path::PathBuf;
use thiserror::Error;

use crate::ffmpeg;

/// Result type for audio operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during audio processing operations.
#[derive(Error, Debug)]
pub enum Error {
    /// The specified audio format is not supported.
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    /// The audio stream was not found in the input file.
    #[error("No audio stream found in {0}")]
    NoAudioStream(PathBuf),

    /// The audio file could not be processed due to invalid data.
    #[error("Invalid audio data in {file}: {reason}")]
    InvalidAudioData {
        /// Path to the file with invalid data.
        file: PathBuf,
        /// Reason why the data is invalid.
        reason: String,
    },

    /// The audio file could not be processed due to a codec issue.
    #[error("Audio codec error: {0}")]
    CodecError(String),

    /// The specified audio track or channel was not found.
    #[error("Audio track {track} not found in {file}")]
    TrackNotFound {
        /// Path to the file.
        file: PathBuf,
        /// The track index that was requested.
        track: usize,
    },

    /// Invalid volume level specified.
    #[error("Invalid volume level: {0}")]
    InvalidVolumeLevel(f64),

    /// Invalid fade duration specified.
    #[error("Invalid fade duration: {0}")]
    InvalidFadeDuration(f64),

    /// Error syncing audio with video.
    #[error("Audio sync error: {0}")]
    SyncError(String),

    /// Error in underlying `FFmpeg` operation.
    #[error("FFmpeg error: {0}")]
    FFmpegError(#[from] ffmpeg::Error),

    /// I/O error occurred during file operations.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Generic audio processing error.
    #[error("Audio processing error: {0}")]
    ProcessingError(String),
}

impl Error {
    /// Creates a new `UnsupportedFormat` error.
    ///
    /// # Arguments
    ///
    /// * `format` - The unsupported format
    ///
    /// # Returns
    ///
    /// A new `Error::UnsupportedFormat`
    #[must_use = "This function returns an error that should be handled"]
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat(format.into())
    }

    /// Creates a new `NoAudioStream` error.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file without an audio stream
    ///
    /// # Returns
    ///
    /// A new `Error::NoAudioStream`
    #[must_use = "This function returns an error that should be handled"]
    pub fn no_audio_stream(path: impl Into<PathBuf>) -> Self {
        Self::NoAudioStream(path.into())
    }

    /// Creates a new `InvalidAudioData` error.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the file with invalid data
    /// * `reason` - Reason why the data is invalid
    ///
    /// # Returns
    ///
    /// A new `Error::InvalidAudioData`
    #[must_use = "This function returns an error that should be handled"]
    pub fn invalid_audio_data(file: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::InvalidAudioData {
            file: file.into(),
            reason: reason.into(),
        }
    }

    /// Creates a new `TrackNotFound` error.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the file
    /// * `track` - The track index that was not found
    ///
    /// # Returns
    ///
    /// A new `Error::TrackNotFound`
    #[must_use = "This function returns an error that should be handled"]
    pub fn track_not_found(file: impl Into<PathBuf>, track: usize) -> Self {
        Self::TrackNotFound {
            file: file.into(),
            track,
        }
    }
}
