/// Error types that can occur during rendering operations.
///
/// This enum represents various errors that can occur during
/// the rendering process, including FFmpeg errors, file system errors,
/// and other rendering-specific errors.
#[derive(Debug, thiserror::Error, Clone)]
pub enum RenderError {
    /// FFmpeg error occurred
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),

    /// File system error occurred
    #[error("File system error: {0}")]
    FileSystemError(String),

    /// IO error occurred
    #[error("IO error: {0}")]
    Io(String),

    /// Invalid configuration error
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Cache error occurred
    #[error("Cache error: {0}")]
    Cache(String),

    /// Rendering cancelled
    #[error("Rendering was cancelled")]
    Cancelled,

    /// Composition error occurred
    #[error("Composition error: {0}")]
    Composition(String),

    /// Timeline error occurred
    #[error("Timeline error: {0}")]
    Timeline(String),

    /// Processing failed
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),

    /// Other rendering error
    #[error("Rendering error: {0}")]
    Other(String),
}

/// Result type for rendering operations.
///
/// This type alias represents the result of a rendering operation,
/// which can either succeed with a value of type T or fail with
/// a `RenderError`.
pub type Result<T> = std::result::Result<T, RenderError>;

/// Simplified result type for rendering operations with no return value.
///
/// This type alias represents the result of a rendering operation,
/// which can either succeed with no value (unit type) or fail with
/// a `RenderError`.
pub type RenderResult = Result<()>;

/// Conversion from std::io::Error to RenderError
impl From<std::io::Error> for RenderError {
    fn from(err: std::io::Error) -> Self {
        RenderError::FileSystemError(err.to_string())
    }
}

/// Conversion from compositor::CompositionError to RenderError
impl From<crate::project::rendering::compositor::CompositionError> for RenderError {
    fn from(err: crate::project::rendering::compositor::CompositionError) -> Self {
        RenderError::Composition(err.to_string())
    }
}
