use crate::cli::output::{OutputFormatter, ProgressReporter};
use crate::cli::utils::TimePosition;
use crate::ffmpeg::{FFmpeg, MediaInfo};
use chrono::{DateTime, Utc};
use mime_guess::MimeGuess;
/// Command definitions and registry for the CLI application.
///
/// This module defines the command trait that all edv commands must implement,
/// as well as the command registry that manages available commands. It serves
/// as the core abstraction for command execution in the application.
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use thiserror::Error;

use crate::core::Context;

use super::{Error, Result};

/// Trait that all commands must implement.
///
/// This trait defines the interface for all commands in the application.
/// Each command must provide its name, description, and an execution method
/// that performs the command's actual functionality.
pub trait Command: Send + Sync + Debug {
    /// Gets the name of the command.
    ///
    /// This name is used for command registration and lookup.
    fn name(&self) -> &str;

    /// Gets a human-readable description of the command.
    ///
    /// This description is used for help text and documentation.
    fn description(&self) -> &str;

    /// Gets usage examples for the command.
    ///
    /// These examples are displayed in help text to guide users.
    fn usage(&self) -> &str;

    /// Executes the command with the given context and arguments.
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context containing configuration and services
    /// * `args` - Command-specific arguments parsed from the command line
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or failure of the command.
    fn execute(&self, context: &Context, args: &[String]) -> Result<()>;
}

/// Registry for managing available commands.
///
/// The command registry maintains a collection of all available commands
/// and provides methods for registering, looking up, and listing commands.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    /// Map of command names to command implementations
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    /// Creates a new, empty command registry.
    ///
    /// # Returns
    ///
    /// A new `CommandRegistry` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Registers a command with the registry.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to register
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or whether the command name was already registered.
    ///
    /// # Errors
    ///
    /// Returns an error if a command with the same name is already registered.
    pub fn register(&mut self, command: Box<dyn Command>) -> Result<()> {
        let name = command.name().to_string();

        if self.commands.contains_key(&name) {
            return Err(Error::DuplicateCommand(name));
        }

        self.commands.insert(name, command);
        Ok(())
    }

    /// Gets a command by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the command to retrieve
    ///
    /// # Returns
    ///
    /// `Result<&dyn Command>` containing a reference to the command if found.
    ///
    /// # Errors
    ///
    /// Returns an error if no command with the given name is registered.
    pub fn get(&self, name: &str) -> Result<&dyn Command> {
        self.commands
            .get(name)
            .map(AsRef::as_ref)
            .ok_or_else(|| Error::UnknownCommand(name.to_string()))
    }

    /// Gets a list of all registered commands.
    ///
    /// # Returns
    ///
    /// A vector of references to all registered commands.
    #[must_use]
    pub fn list(&self) -> Vec<&dyn Command> {
        self.commands.values().map(AsRef::as_ref).collect()
    }

    /// Gets a list of all command names.
    ///
    /// # Returns
    ///
    /// A vector of command names.
    #[must_use]
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Checks if a command with the given name is registered.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the command to check
    ///
    /// # Returns
    ///
    /// `true` if a command with the given name is registered, `false` otherwise.
    #[must_use]
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Gets the number of registered commands.
    ///
    /// # Returns
    ///
    /// The number of commands in the registry.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

/// Renders a project to a video file.
#[derive(Debug)]
pub struct RenderCommand;

impl RenderCommand {
    /// Creates a new render command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for RenderCommand {
    fn name(&self) -> &str {
        "render"
    }

    fn description(&self) -> &str {
        "Renders a project to a video file"
    }

    fn usage(&self) -> &str {
        "render --project <project_file> --output <output_file> [options]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // For now, return a simple not implemented error
        // The actual implementation will be added in a separate PR
        context.logger.info("Render command received");
        context.logger.info(&format!("Args: {:?}", args));

        // Return success for now - this is just a stub until fully implemented
        Ok(())
    }
}

/// Display information about a media file.
#[derive(Debug)]
pub struct InfoCommand;

impl InfoCommand {
    /// Creates a new info command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for InfoCommand {
    fn name(&self) -> &str {
        "info"
    }

    fn description(&self) -> &str {
        "Display information about a media file"
    }

    fn usage(&self) -> &str {
        "info <file_path> [--detailed]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::MissingArgument("file path".to_string()));
        }

        let file_path = &args[0];
        let path = Path::new(file_path);

        // Check if file exists
        if !path.exists() {
            return Err(Error::InvalidPath(format!("File not found: {}", file_path)));
        }

        // Get basic file information
        context
            .logger
            .info(&format!("File information for: {}", file_path));

        if let Ok(metadata) = fs::metadata(path) {
            let size_bytes = metadata.len();
            let size_formatted = format_file_size(size_bytes);

            context.logger.info(&format!("File exists: Yes"));
            context
                .logger
                .info(&format!("Size: {} ({} bytes)", size_formatted, size_bytes));

            // Get and format modification time
            if let Ok(modified) = metadata.modified() {
                let modified: DateTime<Utc> = modified.into();
                context.logger.info(&format!(
                    "Modified: {}",
                    modified.format("%Y-%m-%d %H:%M:%S UTC")
                ));
            }

            // Get MIME type (if can be guessed)
            let mime = MimeGuess::from_path(path).first_or_octet_stream();
            context.logger.info(&format!("Type: {}", mime));

            // Check if it's a media file
            let is_media_file = mime.type_().as_str().starts_with("video")
                || mime.type_().as_str().starts_with("audio")
                || mime.type_().as_str().starts_with("image");

            // Show if it's a directory
            if metadata.is_dir() {
                context.logger.info("Type: Directory");
            } else if metadata.is_file() {
                context.logger.info("Type: Regular file");
            }

            // Show detailed information if requested
            let detailed = args.len() > 1 && (args[1] == "--detailed" || args[1] == "-d");

            // If it's a media file and FFmpeg is available, get media info
            if is_media_file {
                match FFmpeg::detect() {
                    Ok(ffmpeg) => {
                        context.logger.info("Media Information:");

                        match ffmpeg.get_media_info(path) {
                            Ok(media_info) => {
                                display_media_info(&media_info, context, detailed)?;
                            }
                            Err(e) => {
                                context
                                    .logger
                                    .warning(&format!("Could not retrieve media info: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        context
                            .logger
                            .warning(&format!("FFmpeg not available: {e}"));
                        context
                            .logger
                            .warning("Media information cannot be displayed without FFmpeg.");
                    }
                }
            } else if detailed {
                context.logger
                    .info("Note: This is not a media file, so no media-specific information is available.");
            }
        } else {
            return Err(Error::InvalidPath(format!(
                "Could not read file metadata: {}",
                file_path
            )));
        }

        context.logger.info("Info command executed successfully");
        Ok(())
    }
}

/// Displays media information.
///
/// # Arguments
///
/// * `media_info` - The media information to display
/// * `context` - The execution context
/// * `detailed` - Whether to display detailed information
///
/// # Returns
///
/// A Result indicating success or failure
///
/// # Errors
///
/// Returns an error if the operation fails
fn display_media_info(media_info: &MediaInfo, context: &Context, detailed: bool) -> Result<()> {
    // Display format info
    let format = &media_info.format;
    context
        .logger
        .info(&format!("  Format: {}", format.format_long_name));

    // Display duration
    if let Some(duration) = media_info.duration_seconds() {
        let duration_fmt = format_duration(duration);
        context
            .logger
            .info(&format!("  Duration: {}", duration_fmt));
    }

    // Display bit rate
    if let Some(bit_rate) = media_info.bit_rate() {
        context
            .logger
            .info(&format!("  Bit Rate: {} kb/s", bit_rate / 1000));
    }

    // Display video streams
    let video_streams = media_info.video_streams();
    if !video_streams.is_empty() {
        context
            .logger
            .info(&format!("  Video Streams: {}", video_streams.len()));

        for (i, stream) in video_streams.iter().enumerate() {
            context
                .logger
                .info(&format!("    Stream #{}: {}", i, stream.codec_long_name));

            if let (Some(width), Some(height)) = (stream.width, stream.height) {
                context
                    .logger
                    .info(&format!("      Resolution: {}x{}", width, height));
            }

            if let Some(frame_rate) = &stream.frame_rate {
                if let Ok((num, den)) = parse_frame_rate(frame_rate) {
                    let fps = num as f64 / den as f64;
                    context
                        .logger
                        .info(&format!("      Frame Rate: {:.2} fps", fps));
                }
            }

            if detailed {
                if let Some(pixel_format) = &stream.pixel_format {
                    context
                        .logger
                        .info(&format!("      Pixel Format: {}", pixel_format));
                }

                if let Some(bit_rate) = &stream.bit_rate {
                    if let Ok(br) = bit_rate.parse::<u64>() {
                        context
                            .logger
                            .info(&format!("      Bit Rate: {} kb/s", br / 1000));
                    }
                }

                // Display tags if available
                if let Some(tags) = &stream.tags {
                    context.logger.info("      Tags:");
                    for (key, value) in tags {
                        context.logger.info(&format!("        {}: {}", key, value));
                    }
                }
            }
        }
    }

    // Display audio streams
    let audio_streams = media_info.audio_streams();
    if !audio_streams.is_empty() {
        context
            .logger
            .info(&format!("  Audio Streams: {}", audio_streams.len()));

        for (i, stream) in audio_streams.iter().enumerate() {
            context
                .logger
                .info(&format!("    Stream #{}: {}", i, stream.codec_long_name));

            if let Some(sample_rate) = &stream.sample_rate {
                context
                    .logger
                    .info(&format!("      Sample Rate: {} Hz", sample_rate));
            }

            if let Some(channels) = stream.channels {
                context
                    .logger
                    .info(&format!("      Channels: {}", channels));

                if let Some(channel_layout) = &stream.channel_layout {
                    context
                        .logger
                        .info(&format!("      Channel Layout: {}", channel_layout));
                }
            }

            if detailed {
                if let Some(bit_rate) = &stream.bit_rate {
                    if let Ok(br) = bit_rate.parse::<u64>() {
                        context
                            .logger
                            .info(&format!("      Bit Rate: {} kb/s", br / 1000));
                    }
                }

                // Display tags if available
                if let Some(tags) = &stream.tags {
                    context.logger.info("      Tags:");
                    for (key, value) in tags {
                        context.logger.info(&format!("        {}: {}", key, value));
                    }
                }
            }
        }
    }

    // Display subtitle streams if detailed mode
    if detailed {
        let subtitle_streams = media_info.subtitle_streams();
        if !subtitle_streams.is_empty() {
            context
                .logger
                .info(&format!("  Subtitle Streams: {}", subtitle_streams.len()));

            for (i, stream) in subtitle_streams.iter().enumerate() {
                context
                    .logger
                    .info(&format!("    Stream #{}: {}", i, stream.codec_long_name));

                if let Some(tags) = &stream.tags {
                    context.logger.info("      Tags:");
                    for (key, value) in tags {
                        context.logger.info(&format!("        {}: {}", key, value));
                    }
                }
            }
        }

        // Display format tags if available
        if let Some(tags) = &format.tags {
            context.logger.info("  Format Tags:");
            for (key, value) in tags {
                context.logger.info(&format!("    {}: {}", key, value));
            }
        }
    }

    Ok(())
}

/// Formats a file size into a human-readable string.
///
/// # Arguments
///
/// * `size` - The file size in bytes
///
/// # Returns
///
/// A human-readable string representing the file size
fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}

/// Formats a duration in seconds to a human-readable string.
///
/// # Arguments
///
/// * `seconds` - Duration in seconds
///
/// # Returns
///
/// A formatted string representing the duration (e.g., "1h 23m 45s")
fn format_duration(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u64;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u64;
    let secs = (seconds % 60.0).floor() as u64;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{:.2}s", seconds)
    }
}

/// Parses a frame rate string in the format "num/den".
///
/// # Arguments
///
/// * `frame_rate` - The frame rate string
///
/// # Returns
///
/// A Result containing a tuple of (numerator, denominator)
///
/// # Errors
///
/// Returns an error if the string cannot be parsed
fn parse_frame_rate(frame_rate: &str) -> Result<(u64, u64)> {
    let parts: Vec<&str> = frame_rate.split('/').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidArgument(format!(
            "Invalid frame rate format: {frame_rate}"
        )));
    }

    let num = parts[0]
        .parse::<u64>()
        .map_err(|e| Error::InvalidArgument(format!("Invalid frame rate numerator: {e}")))?;

    let den = parts[1]
        .parse::<u64>()
        .map_err(|e| Error::InvalidArgument(format!("Invalid frame rate denominator: {e}")))?;

    if den == 0 {
        return Err(Error::InvalidArgument(
            "Frame rate denominator cannot be zero".to_string(),
        ));
    }

    Ok((num, den))
}

/// Command for trimming a video file.
///
/// This command trims a video file to a specified start and end time,
/// creating a new file with the trimmed content. It supports both
/// stream copy (fast) and re-encoding (higher quality for precise cuts).
#[derive(Debug)]
pub struct TrimCommand;

impl TrimCommand {
    /// Creates a new trim command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Checks if the given file exists and is readable.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to check
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or failure.
    fn check_input_file(&self, path: &str) -> Result<()> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(Error::InvalidArgument(format!(
                "Input file does not exist: {path:?}"
            )));
        }
        if !path.is_file() {
            return Err(Error::InvalidArgument(format!(
                "Not a regular file: {path:?}"
            )));
        }
        Ok(())
    }

    /// Extracts and validates time position arguments.
    ///
    /// # Arguments
    ///
    /// * `start_str` - Optional start time string
    /// * `end_str` - Optional end time string
    /// * `duration` - Media duration for validation
    ///
    /// # Returns
    ///
    /// Result with tuple of (start_pos, end_pos) as TimePosition objects
    fn extract_time_positions(
        &self,
        start_str: Option<&str>,
        end_str: Option<&str>,
        duration: f64,
    ) -> Result<(
        crate::utility::time::TimePosition,
        crate::utility::time::TimePosition,
    )> {
        use crate::utility::time::TimePosition;

        // Parse start time (default to 0)
        let start_pos = if let Some(start) = start_str {
            TimePosition::parse(start)
                .map_err(|e| Error::InvalidArgument(format!("Invalid start time: {e}")))?
        } else {
            TimePosition::zero()
        };

        // Parse end time (default to duration)
        let end_pos = if let Some(end) = end_str {
            TimePosition::parse(end)
                .map_err(|e| Error::InvalidArgument(format!("Invalid end time: {e}")))?
        } else {
            TimePosition::from_seconds(duration)
        };

        // Validate time positions
        if start_pos.as_seconds() >= end_pos.as_seconds() {
            return Err(Error::InvalidArgument(
                "Start time must be less than end time".to_string(),
            ));
        }

        if end_pos.as_seconds() > duration {
            return Err(Error::InvalidArgument(format!(
                "End time ({}) exceeds media duration ({})",
                end_pos,
                format_duration(duration)
            )));
        }

        Ok((start_pos, end_pos))
    }
}

impl Command for TrimCommand {
    fn name(&self) -> &str {
        "trim"
    }

    fn description(&self) -> &str {
        "Trims a video file to specified start and end times"
    }

    fn usage(&self) -> &str {
        "trim --input <input_file> --output <output_file> [--start <time>] [--end <time>] [--recompress]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.len() < 2 {
            return Err(Error::InvalidArgument(
                "Missing required arguments. Use --help for usage information.".to_string(),
            ));
        }

        let input_file = &args[0];
        let output_file = &args[1];

        // Parse remaining arguments
        let mut start_time = None;
        let mut end_time = None;
        let mut recompress = false;

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--start" => {
                    if i + 1 < args.len() {
                        start_time = Some(args[i + 1].as_str());
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument(
                            "--start requires a value".to_string(),
                        ));
                    }
                }
                "--end" => {
                    if i + 1 < args.len() {
                        end_time = Some(args[i + 1].as_str());
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument("--end requires a value".to_string()));
                    }
                }
                "--recompress" => {
                    recompress = true;
                    i += 1;
                }
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "Unknown argument: {}",
                        args[i]
                    )));
                }
            }
        }

        // Validate input file
        self.check_input_file(input_file)?;

        // Initialize FFmpeg
        let ffmpeg = FFmpeg::detect().map_err(|e| Error::FFmpegError(e.to_string()))?;

        // Get media information to validate times
        context
            .logger
            .info(&format!("Analyzing media file: {input_file}"));
        let media_info = ffmpeg
            .get_media_info(input_file)
            .map_err(|e| Error::FFmpegError(format!("Failed to get media info: {e}")))?;

        // Get duration from format section
        let duration_str = media_info.format.duration.as_ref().ok_or_else(|| {
            Error::CommandExecution("Media file has no duration information".to_string())
        })?;

        let duration = duration_str
            .parse::<f64>()
            .map_err(|_| Error::CommandExecution("Invalid duration in media info".to_string()))?;

        // Extract and validate time positions
        let (start_pos, end_pos) = self.extract_time_positions(start_time, end_time, duration)?;

        // Build FFmpeg command for trimming
        context.logger.info(&format!(
            "Trimming from {} to {} (duration: {})",
            start_pos,
            end_pos,
            format_duration(end_pos.as_seconds() - start_pos.as_seconds())
        ));

        let mut ffmpeg_cmd = ffmpeg.command();

        // Add input options
        ffmpeg_cmd.add_input_option("-ss", &start_pos.to_string());

        // Add input file
        ffmpeg_cmd.add_input(input_file);

        // Add duration (more precise than end time)
        let trim_duration = end_pos.as_seconds() - start_pos.as_seconds();
        ffmpeg_cmd.add_output_option("-t", &trim_duration.to_string());

        // Configure output options
        if !recompress {
            // Use stream copy for faster processing
            ffmpeg_cmd.add_output_option("-c", "copy");
        }

        // Set output file
        ffmpeg_cmd.set_output(output_file);

        // Create progress reporter with total duration in seconds
        let mut progress = ProgressReporter::new(trim_duration as usize, "Trimming video", true);

        // Execute FFmpeg command
        ffmpeg_cmd
            .execute_with_progress(|progress_line| {
                // Process progress updates from FFmpeg
                if let Some(time_pos) = extract_time_from_progress(progress_line) {
                    progress.update(time_pos as usize);
                }
            })
            .map_err(|e| Error::FFmpegError(format!("FFmpeg execution failed: {e}")))?;

        progress.complete();
        context
            .logger
            .info(&format!("Trimmed video saved to: {output_file}"));

        Ok(())
    }
}

/// Extracts time position from FFmpeg progress output.
///
/// # Arguments
///
/// * `line` - Progress line from FFmpeg
///
/// # Returns
///
/// Option containing time position in seconds if found
fn extract_time_from_progress(line: &str) -> Option<f64> {
    // Extract time from FFmpeg progress line (e.g., "time=00:00:10.00")
    if let Some(pos) = line.find("time=") {
        let time_str = &line[pos + 5..];
        if let Some(end) = time_str.find(' ') {
            let time_val = &time_str[..end];
            // Handle different time formats (HH:MM:SS.MS or seconds)
            if time_val.contains(':') {
                // Parse time in format HH:MM:SS.MS
                let parts: Vec<&str> = time_val.split(':').collect();
                if parts.len() == 3 {
                    let hours = parts[0].parse::<f64>().unwrap_or(0.0);
                    let minutes = parts[1].parse::<f64>().unwrap_or(0.0);
                    let seconds = parts[2].parse::<f64>().unwrap_or(0.0);
                    return Some(hours * 3600.0 + minutes * 60.0 + seconds);
                }
            } else if let Ok(secs) = time_val.parse::<f64>() {
                return Some(secs);
            }
        }
    }
    None
}

/// Undoes the last edit in a project.
#[derive(Debug)]
pub struct ProjectUndoCommand;

impl ProjectUndoCommand {
    /// Creates a new project undo command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for ProjectUndoCommand {
    fn name(&self) -> &str {
        "project-undo"
    }

    fn description(&self) -> &str {
        "Undoes the last edit in a project"
    }

    fn usage(&self) -> &str {
        "project-undo --project <project_file>"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::MissingArgument("Project file path".to_string()));
        }

        let project_path = &args[0];
        context
            .logger
            .info(&format!("Undoing last edit in project: {project_path}"));

        // Load the project
        let project_result = crate::project::Project::load(&std::path::Path::new(project_path));
        let mut project = match project_result {
            Ok(p) => p,
            Err(e) => {
                return Err(Error::ProjectError(format!("Failed to load project: {e}")));
            }
        };

        // Undo the last edit
        match project.timeline.undo() {
            Ok(_) => {
                context.logger.info("Successfully undid last edit");

                // Save the project with updated state
                if let Err(e) = project.save(&std::path::Path::new(project_path)) {
                    return Err(Error::ProjectError(format!(
                        "Failed to save project after undo: {e}"
                    )));
                }

                context.logger.info("Project saved successfully");
                Ok(())
            }
            Err(e) => Err(Error::ProjectError(format!("Failed to undo: {e}"))),
        }
    }
}

/// Redoes the last undone edit in a project.
#[derive(Debug)]
pub struct ProjectRedoCommand;

impl ProjectRedoCommand {
    /// Creates a new project redo command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for ProjectRedoCommand {
    fn name(&self) -> &str {
        "project-redo"
    }

    fn description(&self) -> &str {
        "Redoes the last undone edit in a project"
    }

    fn usage(&self) -> &str {
        "project-redo --project <project_file>"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::MissingArgument("Project file path".to_string()));
        }

        let project_path = &args[0];
        context.logger.info(&format!(
            "Redoing last undone edit in project: {project_path}"
        ));

        // Load the project
        let project_result = crate::project::Project::load(&std::path::Path::new(project_path));
        let mut project = match project_result {
            Ok(p) => p,
            Err(e) => {
                return Err(Error::ProjectError(format!("Failed to load project: {e}")));
            }
        };

        // Redo the last undone edit
        match project.timeline.redo() {
            Ok(_) => {
                context.logger.info("Successfully redid last undone edit");

                // Save the project with updated state
                if let Err(e) = project.save(&std::path::Path::new(project_path)) {
                    return Err(Error::ProjectError(format!(
                        "Failed to save project after redo: {e}"
                    )));
                }

                context.logger.info("Project saved successfully");
                Ok(())
            }
            Err(e) => Err(Error::ProjectError(format!("Failed to redo: {e}"))),
        }
    }
}

/// Play a video file.
#[derive(Debug)]
pub struct PlayCommand;

impl PlayCommand {
    /// Creates a new play command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validates that the input file exists and is readable.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the file is valid, or an `Error` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or is not readable.
    fn check_input_file(&self, path: &str) -> Result<()> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(Error::InvalidPath(format!(
                "Input file does not exist: {}",
                path.display()
            )));
        }

        if !path.is_file() {
            return Err(Error::InvalidPath(format!(
                "Input path is not a file: {}",
                path.display()
            )));
        }

        // Try to open the file to check if it's readable
        match fs::File::open(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::InvalidPath(format!(
                "Input file is not readable: {}: {}",
                path.display(),
                e
            ))),
        }
    }

    /// Extract and validate time positions for playing.
    ///
    /// # Arguments
    ///
    /// * `start_time` - Optional starting time string
    /// * `end_time` - Optional ending time string
    /// * `duration` - Total duration of the file in seconds
    ///
    /// # Returns
    ///
    /// A tuple of (start, end) positions.
    ///
    /// # Errors
    ///
    /// Returns an error if the time positions are invalid.
    fn extract_time_positions(
        &self,
        start_time: Option<&str>,
        end_time: Option<&str>,
        duration: f64,
    ) -> Result<(TimePosition, TimePosition)> {
        // Default start position is beginning of file
        let start_pos = if let Some(start) = start_time {
            start
                .parse::<TimePosition>()
                .map_err(|e| Error::InvalidTimeFormat(format!("Invalid start time: {start}")))?
        } else {
            TimePosition::from_seconds(0.0)
        };

        // Default end position is end of file
        let end_pos = if let Some(end) = end_time {
            end.parse::<TimePosition>()
                .map_err(|e| Error::InvalidTimeFormat(format!("Invalid end time: {end}")))?
        } else {
            TimePosition::from_seconds(duration)
        };

        // Validate positions
        if start_pos.as_seconds() < 0.0 {
            return Err(Error::InvalidTimeFormat(
                "Start time cannot be negative".to_string(),
            ));
        }

        if end_pos.as_seconds() > duration {
            return Err(Error::InvalidTimeFormat(format!(
                "End time ({}) exceeds file duration ({})",
                end_pos.as_seconds(),
                duration
            )));
        }

        if start_pos.as_seconds() >= end_pos.as_seconds() {
            return Err(Error::InvalidTimeFormat(
                "Start time must be less than end time".to_string(),
            ));
        }

        Ok((start_pos, end_pos))
    }
}

impl Command for PlayCommand {
    fn name(&self) -> &str {
        "play"
    }

    fn description(&self) -> &str {
        "Plays a video file with optional start and end times"
    }

    fn usage(&self) -> &str {
        "play --input <input_file> [--start <time>] [--end <time>]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::InvalidArgument(
                "Missing required arguments. Use --help for usage information.".to_string(),
            ));
        }

        let input_file = &args[0];

        // Parse remaining arguments
        let mut start_time = None;
        let mut end_time = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--start" => {
                    if i + 1 < args.len() {
                        start_time = Some(args[i + 1].as_str());
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument(
                            "--start requires a value".to_string(),
                        ));
                    }
                }
                "--end" => {
                    if i + 1 < args.len() {
                        end_time = Some(args[i + 1].as_str());
                        i += 2;
                    } else {
                        return Err(Error::InvalidArgument("--end requires a value".to_string()));
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

        // Validate input file
        self.check_input_file(input_file)?;

        // Initialize FFmpeg
        let ffmpeg = FFmpeg::detect().map_err(|e| Error::FFmpegError(e.to_string()))?;

        // Get media information
        context
            .logger
            .info(&format!("Analyzing media file: {input_file}"));
        let media_info = ffmpeg
            .get_media_info(input_file)
            .map_err(|e| Error::FFmpegError(format!("Failed to get media info: {e}")))?;

        // Get duration from format section
        let duration_str = media_info.format.duration.as_ref().ok_or_else(|| {
            Error::CommandExecution("Media file has no duration information".to_string())
        })?;

        let duration = duration_str
            .parse::<f64>()
            .map_err(|_| Error::CommandExecution("Invalid duration in media info".to_string()))?;

        // Extract and validate time positions if provided
        let (start_pos, end_pos) = self.extract_time_positions(start_time, end_time, duration)?;

        // Build FFmpeg command for playing
        context.logger.info(&format!(
            "Playing from {:?} to {:?} (duration: {})",
            start_pos,
            end_pos,
            format_duration(end_pos.as_seconds() - start_pos.as_seconds())
        ));

        // Get the ffplay path - try to find it in the same directory as ffmpeg
        let ffmpeg_path = ffmpeg.path();
        let ffplay_path = if cfg!(windows) {
            // On Windows, replace ffmpeg.exe with ffplay.exe
            let mut ffplay = ffmpeg_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf();
            ffplay.push("ffplay.exe");
            ffplay
        } else {
            // On Unix, replace ffmpeg with ffplay
            let mut ffplay = ffmpeg_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf();
            ffplay.push("ffplay");
            ffplay
        };

        // Create command for ffplay
        context
            .logger
            .info(&format!("Using ffplay: {:?}", ffplay_path));
        let mut cmd = std::process::Command::new(ffplay_path);

        // 基本的な設定だけにシンプル化
        cmd.arg("-autoexit");
        cmd.arg("-window_title")
            .arg(format!("EDV Player - {}", input_file));

        // ログレベルを警告のみに設定（静かな出力）
        cmd.arg("-loglevel").arg("warning");

        // 同期オプション - ビデオ優先に変更
        cmd.arg("-sync").arg("video");

        // 入力ファイルを追加（最初に配置）
        cmd.arg(input_file);

        // 開始時間を設定
        cmd.arg("-ss").arg(start_pos.as_seconds().to_string());

        // 継続時間を設定
        let duration = end_pos.as_seconds() - start_pos.as_seconds();
        cmd.arg("-t").arg(duration.to_string());

        // シンプルなログ出力
        context.logger.info("Starting playback...");

        // Execute command
        match cmd.status() {
            Ok(status) => {
                if status.success() {
                    context.logger.info("Playback completed successfully");
                } else {
                    return Err(Error::CommandExecution(
                        "Playback terminated with an error".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(Error::CommandExecution(format!(
                    "Failed to start playback: {e}"
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock command for testing
    #[derive(Debug)]
    struct MockCommand {
        name: String,
        description: String,
        usage: String,
    }

    impl Command for MockCommand {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn usage(&self) -> &str {
            &self.usage
        }

        fn execute(&self, _context: &Context, _args: &[String]) -> Result<()> {
            // Mock implementation that does nothing
            Ok(())
        }
    }

    impl MockCommand {
        fn new(name: &str, description: &str, usage: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
                usage: usage.to_string(),
            }
        }
    }

    #[test]
    fn test_register_and_get_command() {
        let mut registry = CommandRegistry::new();
        let command = MockCommand::new("test", "Test command", "test --arg value");
        let command_name = command.name().to_string();

        // Register the command
        registry.register(Box::new(command)).unwrap();

        // Verify command is in registry
        assert!(registry.has_command(&command_name));
        assert_eq!(registry.command_count(), 1);

        // Get the command
        let cmd = registry.get(&command_name).unwrap();
        assert_eq!(cmd.name(), "test");
        assert_eq!(cmd.description(), "Test command");
        assert_eq!(cmd.usage(), "test --arg value");
    }

    #[test]
    fn test_duplicate_command_registration() {
        let mut registry = CommandRegistry::new();
        let command1 = MockCommand::new("test", "First test command", "test1");
        let command2 = MockCommand::new("test", "Second test command", "test2");

        // Register the first command
        registry.register(Box::new(command1)).unwrap();

        // Try to register a command with the same name
        let result = registry.register(Box::new(command2));
        assert!(result.is_err());

        // Ensure only one command is registered
        assert_eq!(registry.command_count(), 1);
    }

    #[test]
    fn test_unknown_command() {
        let registry = CommandRegistry::new();

        // Try to get a non-existent command
        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_commands() {
        let mut registry = CommandRegistry::new();
        let command1 = MockCommand::new("test1", "First test command", "test1");
        let command2 = MockCommand::new("test2", "Second test command", "test2");

        // Register commands
        registry.register(Box::new(command1)).unwrap();
        registry.register(Box::new(command2)).unwrap();

        // List commands
        let commands = registry.list();
        assert_eq!(commands.len(), 2);

        // Get command names
        let names = registry.command_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"test1".to_string()));
        assert!(names.contains(&"test2".to_string()));
    }
}
