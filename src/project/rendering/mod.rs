mod compositor;
mod config;
/// Timeline rendering functionality.
///
/// This module provides functionality for rendering timeline projects to
/// video files, with support for multi-track composition, effects, and
/// progress monitoring.
mod pipeline;
mod progress;

pub use compositor::{CompositionError, TrackCompositor};
pub use config::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
pub use pipeline::{RenderPipeline, RenderResult, render_project, render_project_simple};
pub use progress::{ProgressCallback, RenderProgress};

/// Error types for rendering operations.
#[derive(Debug, thiserror::Error, Clone)]
pub enum RenderError {
    /// Error during FFmpeg operations.
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),

    /// Error during track composition.
    #[error("Composition error: {0}")]
    Composition(String),

    /// Error accessing files.
    #[error("I/O error: {0}")]
    Io(String),

    /// Error with the timeline structure.
    #[error("Timeline error: {0}")]
    Timeline(String),

    /// Rendering was cancelled by the user.
    #[error("Rendering cancelled by user")]
    Cancelled,
}

impl From<std::io::Error> for RenderError {
    fn from(err: std::io::Error) -> Self {
        RenderError::Io(err.to_string())
    }
}

impl From<crate::ffmpeg::Error> for RenderError {
    fn from(err: crate::ffmpeg::Error) -> Self {
        RenderError::FFmpeg(err.to_string())
    }
}

impl From<compositor::CompositionError> for RenderError {
    fn from(err: compositor::CompositionError) -> Self {
        RenderError::Composition(err.to_string())
    }
}

/// Type alias for render operation results.
pub type Result<T> = std::result::Result<T, RenderError>;
