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
    IoError(String),

    /// Error parsing `FFmpeg` output.
    #[error("Error parsing FFmpeg output: {0}")]
    ParseError(String),

    /// Missing output file specification.
    #[error("No output file specified")]
    MissingOutput,

    /// Missing input file specification.
    #[error("No input files specified")]
    MissingInput,

    /// Process execution error.
    #[error("Process error: {0}")]
    ProcessError(String),
}

/// Result type for `FFmpeg` operations.
pub type Result<T> = std::result::Result<T, Error>;
