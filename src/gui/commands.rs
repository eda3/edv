use std::path::Path;

use crate::cli::Command;
use crate::cli::Error;
use crate::cli::Result;
use crate::core::Context;
use crate::ffmpeg::FFmpeg;

use super::frame_player::FramePlayer;

/// Command to play videos with a simple GUI player.
#[derive(Debug)]
pub struct GuiPlayCommand;

impl GuiPlayCommand {
    /// Creates a new GUI play command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for GuiPlayCommand {
    fn name(&self) -> &str {
        "gui-play"
    }

    fn description(&self) -> &str {
        "Plays a video file with a simple GUI player"
    }

    fn usage(&self) -> &str {
        "gui-play --input <input_file> [--width <width>] [--height <height>] [--fps <frame_rate>]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::MissingArgument("Input file path".to_string()));
        }

        let input_file = &args[0];
        context
            .logger
            .info(&format!("GUI Player: playing {input_file}"));

        // Parse remaining arguments
        let mut width = 800;
        let mut height = 600;
        let mut fps = 30.0;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--width" => {
                    if i + 1 < args.len() {
                        width = args[i + 1].parse::<u32>().map_err(|_| {
                            Error::InvalidArgument(format!("Invalid width: {}", args[i + 1]))
                        })?;
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument(
                            "--width requires a value".to_string(),
                        ));
                    }
                }
                "--height" => {
                    if i + 1 < args.len() {
                        height = args[i + 1].parse::<u32>().map_err(|_| {
                            Error::InvalidArgument(format!("Invalid height: {}", args[i + 1]))
                        })?;
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument(
                            "--height requires a value".to_string(),
                        ));
                    }
                }
                "--fps" => {
                    if i + 1 < args.len() {
                        fps = args[i + 1].parse::<f64>().map_err(|_| {
                            Error::InvalidArgument(format!("Invalid fps: {}", args[i + 1]))
                        })?;
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument("--fps requires a value".to_string()));
                    }
                }
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "Unknown argument: {}",
                        args[i]
                    )));
                }
            }
        }

        // Check if input file exists
        let path = Path::new(input_file);
        if !path.exists() {
            return Err(Error::InvalidPath(format!(
                "File not found: {}",
                input_file
            )));
        }

        // Initialize FFmpeg
        let ffmpeg = FFmpeg::detect()
            .map_err(|e| Error::FFmpegError(format!("FFmpeg detection failed: {e}")))?;

        // Create and configure player
        let mut player = FramePlayer::new(ffmpeg);
        player
            .set_title(&format!(
                "EDV Player - {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ))
            .set_frame_size(width, height)
            .set_frame_rate(fps);

        context.logger.info(&format!(
            "Starting GUI player ({}x{} @ {} fps)",
            width, height, fps
        ));

        // Play the video
        match player.play(path) {
            Ok(_) => {
                context.logger.info("GUI player closed successfully");
                Ok(())
            }
            Err(e) => Err(Error::CommandExecution(format!("GUI player error: {e}"))),
        }
    }
}
