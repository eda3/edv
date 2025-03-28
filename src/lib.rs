/// EDV - A video editing library.
///
/// EDV provides a simple, fast, and flexible way to edit and process
/// video and audio files. It is designed to be used both as a library
/// and as a command line tool.
///
/// # Features
///
/// - Video editing: cut, join, resize, convert
/// - Audio processing: volume adjustment, extraction, replacement
/// - Subtitle support: `SRT`, `WebVTT`, styling, positioning
/// - Timeline editing: multi-track, effects, transitions
///
/// # Examples
///
/// ```
/// use edv::audio::volume::adjust_volume;
/// use edv::subtitle::editor::SubtitleEditor;
/// use edv::subtitle::format::SubtitleFormat;
/// ```
// Export modules
pub mod audio;
pub mod cli;
pub mod core;
pub mod ffmpeg;
pub mod processing;
pub mod project;
pub mod subtitle;
pub mod utility;

// Reexport main types for convenience
pub use audio::error::{Error as AudioError, Result as AudioResult};
pub use core::{Error as CoreError, Result as CoreResult};
pub use ffmpeg::error::{Error as FFmpegError, Result as FFmpegResult};
pub use project::{Project, ProjectError, Result as ProjectResult};
pub use subtitle::error::{Error as SubtitleError, Result as SubtitleResult};
