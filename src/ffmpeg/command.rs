/// FFmpeg command construction utilities.
///
/// This module provides a simplified interface for building FFmpeg commands.

use std::path::PathBuf;
use crate::ffmpeg::FFmpeg;

/// Represents an FFmpeg command.
#[derive(Debug, Clone)]
pub struct FFmpegCommand {
    /// The FFmpeg instance to use.
    ffmpeg: FFmpeg,
}

impl FFmpegCommand {
    /// Creates a new FFmpeg command.
    ///
    /// # Arguments
    ///
    /// * `ffmpeg` - The FFmpeg instance to use
    #[must_use]
    pub fn new(ffmpeg: FFmpeg) -> Self {
        Self { ffmpeg }
    }
}
