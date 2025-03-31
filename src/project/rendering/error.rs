/// Error types that can occur during rendering operations.
///
/// This enum represents various errors that can occur during
/// the rendering process, including FFmpeg errors, file system errors,
/// and other rendering-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// FFmpeg error occurred
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),

    /// File system error occurred
    #[error("File system error: {0}")]
    FileSystemError(#[from] std::io::Error),

    /// Invalid configuration error
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Rendering cancelled
    #[error("Rendering was cancelled")]
    Cancelled,

    /// Other rendering error
    #[error("Rendering error: {0}")]
    Other(String),
}

/// Result type for rendering operations.
///
/// This type alias represents the result of a rendering operation,
/// which can either succeed with no value (unit type) or fail with
/// a `RenderError`.
pub type RenderResult = Result<(), RenderError>;
