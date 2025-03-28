/// Error types for `FFmpeg` operations.
///
/// This module defines the error types and result type for `FFmpeg` operations.
use std::io;
use thiserror::Error;

/// Error type for `FFmpeg` operations.
#[derive(Error, Debug)]
pub enum Error {
    /// `FFmpeg` executable not found.
    #[error("FFmpeg executable not found")]
    NotFound,

    /// `FFmpeg` process timeout.
    #[error("FFmpeg process timed out")]
    Timeout,

    /// `FFmpeg` error output.
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// Error parsing `FFmpeg` output.
    #[error("Error parsing FFmpeg output: {0}")]
    ParseError(String),
}

/// Result type for `FFmpeg` operations.
pub type Result<T> = std::result::Result<T, Error>;
