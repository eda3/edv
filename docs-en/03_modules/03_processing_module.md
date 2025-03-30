# edv - Processing Module Implementation

This document provides detailed implementation guidelines for the Processing module of the edv application, which handles video processing operations through FFmpeg integration.

## Overview

The Processing module serves as the primary interface for video processing operations. It abstracts the complexity of FFmpeg command generation, handles execution, and provides progress reporting for video operations.

## Structure

```
src/
├── ffmpeg/                  // FFmpeg integration core
│   ├── mod.rs               // FFmpeg detection, version handling
│   ├── command.rs           // FFmpeg command builder
│   └── error.rs             // FFmpeg-specific errors
└── processing/              // Processing utilities
    ├── mod.rs               // Module exports and re-exports
    └── ffmpeg_command.rs    // Higher-level command builder
```

## Key Components

### FFmpeg Integration (ffmpeg/mod.rs)

The FFmpeg integration handles detection, validation, and interaction with FFmpeg:

```rust
/// Represents a detected `FFmpeg` installation.
#[derive(Debug, Clone)]
pub struct FFmpeg {
    /// The path to the `FFmpeg` executable.
    path: PathBuf,
    /// The `FFmpeg` version.
    version: Version,
}

impl FFmpeg {
    /// The minimum supported `FFmpeg` version.
    pub const MIN_VERSION: Version = Version {
        major: 4,
        minor: 0,
        patch: 0,
    };

    /// Creates a new `FFmpeg` instance.
    #[must_use]
    pub fn new(path: PathBuf, version: Version) -> Self {
        Self { path, version }
    }

    /// Gets the path to the `FFmpeg` executable.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets the `FFmpeg` version.
    #[must_use]
    pub fn version(&self) -> &Version {
        &self.version
    }
    
    /// Detects the `FFmpeg` installation.
    pub fn detect() -> Result<Self> {
        // First try to find in PATH
        if let Ok(ffmpeg) = Self::detect_in_path() {
            return Ok(ffmpeg);
        }

        // Then try common installation directories
        if let Ok(ffmpeg) = Self::detect_in_common_locations() {
            return Ok(ffmpeg);
        }

        Err(Error::NotFound)
    }
    
    /// Detects `FFmpeg` installation from a specified path.
    pub fn detect_at_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Implementation details...
    }
}
```

### FFmpeg Command Builder (ffmpeg/command.rs)

The command builder creates FFmpeg command lines:

```rust
/// Represents an `FFmpeg` command.
#[derive(Debug, Clone)]
pub struct FFmpegCommand<'a> {
    /// The `FFmpeg` instance to use.
    ffmpeg: &'a FFmpeg,
    /// Input options to apply before specifying inputs.
    input_options: Vec<String>,
    /// Input files for the command.
    inputs: Vec<PathBuf>,
    /// Filter complex to apply (if any).
    filter_complex: Option<String>,
    /// Output options to apply before specifying output.
    output_options: Vec<String>,
    /// Output file for the command.
    output: Option<PathBuf>,
    /// Whether to overwrite output file if it exists.
    overwrite: bool,
}

impl<'a> FFmpegCommand<'a> {
    /// Creates a new `FFmpeg` command.
    #[must_use]
    pub fn new(ffmpeg: &'a FFmpeg) -> Self {
        Self {
            ffmpeg,
            input_options: Vec::new(),
            inputs: Vec::new(),
            filter_complex: None,
            output_options: Vec::new(),
            output: None,
            overwrite: false,
        }
    }

    /// Adds input options to be applied before an input file.
    pub fn input_options<S: AsRef<str>, I: IntoIterator<Item = S>>(
        &mut self,
        options: I,
    ) -> &mut Self {
        self.input_options
            .extend(options.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Adds an input file to the command.
    pub fn input<P: AsRef<Path>>(&mut self, input: P) -> &mut Self {
        self.inputs.push(input.as_ref().to_path_buf());
        self
    }

    /// Sets a filter complex for the command.
    pub fn filter_complex<S: AsRef<str>>(&mut self, filter: S) -> &mut Self {
        self.filter_complex = Some(filter.as_ref().to_string());
        self
    }

    /// Adds output options to be applied before the output file.
    pub fn output_options<S: AsRef<str>, I: IntoIterator<Item = S>>(
        &mut self,
        options: I,
    ) -> &mut Self {
        self.output_options
            .extend(options.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Sets the output file for the command.
    pub fn output<P: AsRef<Path>>(&mut self, output: P) -> &mut Self {
        self.output = Some(output.as_ref().to_path_buf());
        self
    }

    /// Sets whether to overwrite the output file if it exists.
    pub fn overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }
    
    /// Executes the `FFmpeg` command.
    pub fn execute(&self) -> Result<()> {
        // Implementation details...
    }
}
```

### High-Level Command API (processing/ffmpeg_command.rs)

Higher-level API for building FFmpeg commands:

   ```rust
impl<'a> FFmpegCommand<'a> {
    /// Creates a new FFmpegCommand with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_file: None,
            output_file: None,
            overwrite: false,
            video_codec: None,
            audio_codec: None,
            format: None,
            video_filters: vec![],
            audio_filters: vec![],
            seek: None,
            duration: None,
            custom_options: vec![],
        }
    }

    /// Adds an input file to the command.
    #[must_use]
    pub fn with_input(mut self, input: &'a str) -> Self {
        self.input_file = Some(input);
        self
    }

    /// Adds an output file to the command.
    #[must_use]
    pub fn with_output(mut self, output: &'a str) -> Self {
        self.output_file = Some(output);
        self
    }

    /// Sets the overwrite flag.
    #[must_use]
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Sets the video codec.
    #[must_use]
    pub fn with_video_codec(mut self, codec: &'a str) -> Self {
        self.video_codec = Some(codec);
        self
    }

    /// Sets the audio codec.
    #[must_use]
    pub fn with_audio_codec(mut self, codec: &'a str) -> Self {
        self.audio_codec = Some(codec);
        self
    }

    /// Sets the output format.
    #[must_use]
    pub fn with_format(mut self, format: &'a str) -> Self {
        self.format = Some(format);
        self
    }

    /// Adds a video filter.
    #[must_use]
    pub fn with_video_filter(mut self, filter: &'a str) -> Self {
        self.video_filters.push(filter);
        self
    }

    /// Adds an audio filter.
    #[must_use]
    pub fn with_audio_filter(mut self, filter: &'a str) -> Self {
        self.audio_filters.push(filter);
        self
    }

    /// Sets the seek position.
    #[must_use]
    pub fn with_seek(mut self, seek: &'a str) -> Self {
        self.seek = Some(seek);
        self
    }

    /// Sets the duration.
    #[must_use]
    pub fn with_duration(mut self, duration: &'a str) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Builds the FFmpeg command as a vector of arguments.
    #[must_use]
    pub fn build(&self) -> Vec<String> {
        // Implementation details...
    }
}
```

#### Best Practices for Command Builder Usage

The `FFmpegCommand` API provides two different styles of command building:

1. **Low-level API with mutable references**

   The low-level API in `ffmpeg/command.rs` uses borrowing patterns:
```rust
   let mut cmd = FFmpegCommand::new(&ffmpeg);
   cmd.input(input_path)
      .output_options(&output_options)
      .output(output_path)
      .overwrite(true);
   
   cmd.execute()?;
   ```

2. **High-level API with builder pattern**
   
   The high-level API in `processing/ffmpeg_command.rs` uses the builder pattern:
```rust
   let cmd = FFmpegCommand::new()
      .with_input(input_path)
      .with_output(output_path)
      .with_video_codec("libx264")
      .with_audio_codec("aac")
      .with_overwrite(true);
   
   // Use cmd to build the actual command
   ```

Choose the appropriate API based on your needs:
- Use the low-level API when you need fine-grained control over command construction
- Use the high-level API for common operations with a more fluent interface

## Error Handling

Error handling is implemented through specific error types:

```rust
/// Errors that can occur in the `FFmpeg` module.
#[derive(Error, Debug)]
pub enum Error {
    /// `FFmpeg` executable not found.
    #[error("FFmpeg executable not found")]
    NotFound,

    /// `FFmpeg` executable path is not valid.
    #[error("FFmpeg path is not valid: {0}")]
    InvalidPath(String),

    /// `FFmpeg` version is not supported.
    #[error("FFmpeg version {actual} is not supported (minimum: {required})")]
    UnsupportedVersion {
        /// The actual `FFmpeg` version detected.
        actual: Version,
        /// The minimum required `FFmpeg` version.
        required: Version,
    },

    /// Error executing `FFmpeg` command.
    #[error("Error executing FFmpeg command: {0}")]
    ExecutionError(String),

    /// Error parsing `FFmpeg` output.
    #[error("Error parsing FFmpeg output: {0}")]
    OutputParseError(String),

    /// IO error occurred.
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// `FFmpeg` process terminated with non-zero exit code.
    #[error("FFmpeg process terminated: {message}")]
    ProcessTerminated {
        /// The exit code of the process, if available.
        exit_code: Option<i32>,
        /// The error message.
        message: String,
    },

    /// Invalid time format provided.
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),

    /// Missing required argument.
    #[error("Missing argument: {0}")]
    MissingArgument(String),

    /// Invalid argument provided.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}
```

## Integration with Other Modules

### Core Module Integration

The Processing module integrates with the Core module for configuration and logging:

```rust
// Example of integrating with the Core module
pub fn initialize_processing(config: &Config, logger: &dyn Logger) -> Result<ProcessingContext> {
    // Detect FFmpeg or use configured path
    let ffmpeg = if let Some(path) = &config.ffmpeg_path {
        FFmpeg::detect_at_path(path)?
    } else {
        FFmpeg::detect()?
    };
    
    logger.info(&format!("Using FFmpeg version {}", ffmpeg.version()));
    
    Ok(ProcessingContext {
        ffmpeg,
        config: config.clone(),
    })
}
```

### CLI Module Integration

The Processing module integrates with the CLI module for command execution:

```rust
// Example of integrating with the CLI module
pub fn execute_trim_command(args: &TrimArgs, context: &Context) -> Result<()> {
    let ffmpeg = FFmpeg::detect()?;
    
    // Create command
    let cmd = FFmpegCommand::new()
        .with_input(&args.input)
        .with_output(&args.output);
    
    // Add time options if provided
    let cmd = if let Some(start) = &args.start {
        cmd.with_seek(start)
    } else {
        cmd
    };
    
    let cmd = if let Some(duration) = &args.duration {
        cmd.with_duration(duration)
    } else {
        cmd
    };
    
    // Execute command
    // (would need to convert to low-level command)
    // ...
    
    Ok(())
}
```

## Implementation Status Update (2024)

### Current Implementation Status

The Processing module has been significantly refactored and improved:

| Component | Status | Notes |
|-----------|--------|-------|
| FFmpeg Detection | ✅ Complete | Robust FFmpeg detection with version validation |
| Command Builder | ✅ Complete | Both low-level and high-level APIs implemented |
| Error Handling | ✅ Complete | Comprehensive error types with detailed messages |
| Basic Operations | 🔄 In Progress | Core trim, concat functionality implemented |
| Media Info | 🔄 In Progress | Basic media information extraction |
| Advanced Filters | 🔶 Planned | Design completed, implementation coming soon |
| Progress Reporting | 🔶 Planned | Design completed, implementation coming soon |

### Recent Improvements

Several significant improvements have been made to the Processing module:

#### 1. FFmpeg Detection Enhancements

The FFmpeg detection logic has been improved to:
- Search in system PATH
- Check common installation directories based on OS
- Validate FFmpeg version compatibility
- Provide clear error messages when FFmpeg is not found or incompatible

#### 2. Dual Command Building APIs

Two complementary command building APIs have been implemented:
- Low-level API with fine-grained control using borrowing patterns
- High-level API with fluent interface using builder pattern

#### 3. Error Handling Improvements

Error handling has been enhanced with:
- Detailed error types using thiserror
- Clear error messages with context
- Proper error propagation
- Recovery strategies for common error conditions

### Future Development Plans

The following enhancements are planned for the Processing module:

1. **Media Information Extraction**
   - Comprehensive metadata extraction
   - Stream information analysis
   - Format detection

2. **Progress Reporting**
   - Real-time progress updates
   - Time remaining estimation
   - Cancellation support

3. **Advanced Filtering**
   - Video filter graph construction
   - Audio filter support
   - Complex filter chain building

4. **Operation Framework**
   - Standardized operation interface
   - Common operation implementations (trim, concat, etc.)
   - Custom operation support

The Processing module will continue to evolve as a core component of the edv application, providing robust media processing capabilities through FFmpeg integration. 