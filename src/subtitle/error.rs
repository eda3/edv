/// Error types for the subtitle processing module.
///
/// This module defines error types specific to subtitle processing operations.
/// It uses thiserror for ergonomic error definitions and handling.
use std::path::PathBuf;
use thiserror::Error;

use crate::ffmpeg;
use crate::subtitle::format::TimePosition;

/// Result type for subtitle operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during subtitle processing operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Error when parsing subtitle files
    #[error("Failed to parse subtitle file: {reason}")]
    ParseError {
        /// Path to the subtitle file
        file: Option<PathBuf>,
        /// The reason for the parsing failure
        reason: String,
    },

    /// Error when an invalid subtitle format is provided
    #[error("Invalid subtitle format: {0}")]
    InvalidSubtitleFormat(String),

    /// Error when subtitle format is unknown
    #[error("Unknown subtitle format")]
    UnknownFormat,

    /// Error when subtitle parser is not implemented for a format
    #[error("Subtitle parser not implemented for format: {0}")]
    UnsupportedParserFormat(String),

    /// Error when subtitle export is not implemented for a format
    #[error("Subtitle export not implemented for format: {0}")]
    UnsupportedExportFormat(String),

    /// Error when working with subtitle timing
    #[error("Timing error: {0}")]
    TimingError(String),

    /// Error when working with subtitle formatting
    #[error("Formatting error: {0}")]
    FormattingError(String),

    /// Error when encoding or decoding subtitles
    #[error("Encoding error: {0}")]
    EncodingError(String),

    /// Error when an internal I/O operation fails
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Error when there's an invalid time range (start >= end)
    #[error("Invalid time range: start {start:?} >= end {end:?}")]
    InvalidTimeRange {
        /// Start time
        start: TimePosition,
        /// End time
        end: TimePosition,
    },

    /// Error when a split point is outside the subtitle's duration
    #[error("Invalid split point {split_time:?}, must be between {start:?} and {end:?}")]
    InvalidSplitPoint {
        /// The time to split at
        split_time: TimePosition,
        /// Start time of the subtitle
        start: TimePosition,
        /// End time of the subtitle
        end: TimePosition,
    },

    /// Error when trying to save a subtitle with no file path specified
    #[error("No file path specified for saving")]
    NoFilePath,

    /// Error rendering subtitle.
    #[error("Failed to render subtitle: {0}")]
    RenderError(String),

    /// Error burning subtitles into video.
    #[error("Failed to burn subtitles into video: {0}")]
    BurnError(String),

    /// Error in underlying `FFmpeg` operation.
    #[error("FFmpeg error: `{0}`")]
    FFmpegError(#[from] ffmpeg::Error),
}

impl Error {
    /// Creates a parse error with the given file and reason.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the subtitle file
    /// * `reason` - The reason for the parsing failure
    #[must_use]
    pub fn parse_error(file: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::ParseError {
            file: Some(file.into()),
            reason: reason.into(),
        }
    }

    /// Creates a parse error with only a reason.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for the parsing failure
    #[must_use]
    pub fn parse_error_with_reason(reason: impl Into<String>) -> Self {
        Self::ParseError {
            file: None,
            reason: reason.into(),
        }
    }

    /// Creates a timing error with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message
    #[must_use]
    pub fn timing_error(message: impl Into<String>) -> Self {
        Self::TimingError(message.into())
    }

    /// Creates a formatting error with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message
    #[must_use]
    pub fn formatting_error(message: impl Into<String>) -> Self {
        Self::FormattingError(message.into())
    }

    /// Creates an encoding error with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message
    #[must_use]
    pub fn encoding_error(message: impl Into<String>) -> Self {
        Self::EncodingError(message.into())
    }

    /// Creates an invalid subtitle format error.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message
    #[must_use]
    pub fn invalid_subtitle_format(message: impl Into<String>) -> Self {
        Self::InvalidSubtitleFormat(message.into())
    }

    /// Creates an unknown subtitle format error.
    #[must_use]
    pub fn unknown_subtitle_format() -> Self {
        Self::UnknownFormat
    }

    /// Creates an unsupported parser format error.
    ///
    /// # Arguments
    ///
    /// * `format` - The unsupported format
    #[must_use]
    pub fn unsupported_parser_format(format: impl Into<String>) -> Self {
        Self::UnsupportedParserFormat(format.into())
    }

    /// Creates an unsupported export format error.
    ///
    /// # Arguments
    ///
    /// * `format` - The unsupported format
    #[must_use]
    pub fn unsupported_export_format(format: impl Into<String>) -> Self {
        Self::UnsupportedExportFormat(format.into())
    }

    /// Creates an invalid time range error.
    ///
    /// # Arguments
    ///
    /// * `start` - Start time
    /// * `end` - End time
    #[must_use]
    pub fn invalid_time_range(start: TimePosition, end: TimePosition) -> Self {
        Self::InvalidTimeRange { start, end }
    }

    /// Creates an invalid split point error.
    ///
    /// # Arguments
    ///
    /// * `split_time` - The time to split at
    /// * `start` - Start time of the subtitle
    /// * `end` - End time of the subtitle
    #[must_use]
    pub fn invalid_split_point(
        split_time: TimePosition,
        start: TimePosition,
        end: TimePosition,
    ) -> Self {
        Self::InvalidSplitPoint {
            split_time,
            start,
            end,
        }
    }

    /// Creates a no file path error.
    #[must_use]
    pub fn no_file_path() -> Self {
        Self::NoFilePath
    }

    /// Creates a new `RenderError`.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of the render error
    ///
    /// # Returns
    ///
    /// A new `Error::RenderError`
    #[must_use]
    pub fn render_error(message: impl Into<String>) -> Self {
        Self::RenderError(message.into())
    }

    /// Creates a new `BurnError`.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of the burn error
    ///
    /// # Returns
    ///
    /// A new `Error::BurnError`
    #[must_use]
    pub fn burn_error(message: impl Into<String>) -> Self {
        Self::BurnError(message.into())
    }
}
