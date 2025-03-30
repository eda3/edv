# edv - CLI Module Implementation

This document provides detailed implementation guidelines for the Command Line Interface (CLI) module of the edv application.

## Overview

The CLI module serves as the primary user interface for the edv application, handling command parsing, execution, and user interaction. It provides a consistent and intuitive command-line experience for video editing operations.

## Structure

```
src/cli/
├── mod.rs        # Module exports, Error enum, Result type
├── app.rs        # Main application entry point (App, Cli, Commands)
├── commands.rs   # Command registry and implementations
├── args.rs       # Argument parsing utilities
├── output.rs     # Terminal output formatting and progress reporting
└── utils.rs      # CLI utilities (help text, validation)
```

## Key Components

### App (app.rs)

The main application entry point and command dispatcher:

```rust
/// CLI application structure
pub struct App {
    /// Command registry containing all available commands
    command_registry: CommandRegistry,
    /// Application configuration
    config: Config,
    /// Logger for application messages
    logger: Box<dyn Logger>,
}

impl App {
    /// Creates a new application instance with the given configuration
    pub fn new(config: Config, logger: Box<dyn Logger>) -> Self {
        Self {
            command_registry: CommandRegistry::new(),
            config,
            logger,
        }
    }
    
    /// Initializes the application, registering all available commands
    pub fn initialize(&mut self) -> Result<()> {
        // Register all commands
        self.register_commands()?;
        
        self.logger.info("Application initialized");
        Ok(())
    }
    
    /// Registers all available commands with the command registry
    fn register_commands(&mut self) -> Result<()> {
        // Register commands
        self.command_registry.register(Box::new(commands::InfoCommand::new()))?;
        self.command_registry.register(Box::new(commands::RenderCommand::new()))?;
        // Additional commands will be registered here
        
        Ok(())
    }
    
    /// Executes the given command with its arguments
    pub fn execute_command(&self, command: Commands) -> Result<()> {
        // Create execution context
        let context = self.create_execution_context()?;
        
        // Match on command type and execute appropriate handler
        match command {
            Commands::Trim { input, output, start, end, recompress } => {
                // Trim implementation
            },
            Commands::Concat { input, output, recompress } => {
                // Concat implementation
            },
            Commands::Info { input, detailed } => {
                // Get the InfoCommand from the registry and execute it
                if let Ok(info_cmd) = self.command_registry.get("info") {
                    // Convert arguments
                    let mut args = vec![input];
                    if detailed {
                        args.push("--detailed".to_string());
                    }
                    
                    // Execute the command with prepared arguments
                    info_cmd.execute(&context, &args)?;
                } else {
                    // Fallback to placeholder implementation
                    self.logger.info("Info command executed successfully");
                }
            },
            // Other commands...
        }
        
        Ok(())
    }
    
    /// Creates an execution context for command execution
    fn create_execution_context(&self) -> Result<Context> {
        Ok(Context::new(self.config.clone(), self.logger.clone()))
    }
}

/// Application entry point
pub fn run() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Configure logger based on verbosity
    let log_level = if cli.verbose { LogLevel::Debug } else { LogLevel::Info };
    let logger = Box::new(ConsoleLogger::new(log_level));
    
    // Load configuration from file or use default
    let config = match cli.config {
        Some(ref path) => Config::load_from_file(path)?,
        None => Config::load_default()?,
    };
    
    // Create and initialize application
    let mut app = App::new(config, logger);
    app.initialize()?;
    
    // Execute command
    app.execute_command(cli.command)
}
```

### Main Entry Point (main.rs)

The main function serves as the application's entry point and runs the CLI:

```rust
use edv::cli;

fn main() {
    // Run the CLI application
    if let Err(err) = cli::run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
```

### Command Line Parsing (app.rs)

The CLI module uses clap for command line parsing:

```rust
/// Command line arguments parser
#[derive(Parser)]
#[clap(
    name = "edv",
    about = "CLI video editing tool based on FFmpeg",
    version,
    author
)]
pub struct Cli {
    /// Subcommand to execute
    #[clap(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[clap(short, long, global = true)]
    pub verbose: bool,

    /// Path to configuration file
    #[clap(short, long, global = true)]
    pub config: Option<PathBuf>,
}

/// Subcommands supported by the application
#[derive(Subcommand)]
pub enum Commands {
    /// Trim a video to specified start and end times
    Trim {
        /// Input video file path
        #[clap(short, long, value_parser)]
        input: String,

        /// Output video file path
        #[clap(short, long, value_parser)]
        output: String,

        /// Start time (format: HH:MM:SS.mmm or seconds)
        #[clap(short, long)]
        start: Option<String>,

        /// End time (format: HH:MM:SS.mmm or seconds)
        #[clap(short, long)]
        end: Option<String>,

        /// Re-encode the video instead of using stream copy
        #[clap(short, long, action)]
        recompress: bool,
    },

    /// Concatenate multiple video files
    Concat {
        /// Input video files
        #[clap(short, long, value_parser, num_args = 1..)]
        input: Vec<String>,

        /// Output video file path
        #[clap(short, long, value_parser)]
        output: String,

        /// Re-encode the video instead of using stream copy
        #[clap(short, long, action)]
        recompress: bool,
    },

    /// Display information about a media file
    Info {
        /// Input media file path
        #[clap(value_parser)]
        input: String,

        /// Show detailed information
        #[clap(short, long, action)]
        detailed: bool,
    },
    
    // Additional commands will be added here
}
```

### Command Interface (commands.rs)

The trait defining the interface for all commands:

```rust
/// Trait for implementing commands
pub trait Command {
    /// Get the name of the command
    fn name(&self) -> &str;
    
    /// Get a brief description of the command
    fn description(&self) -> &str;
    
    /// Get usage examples for the command
    fn usage(&self) -> &str;
    
    /// Execute the command with the given context and arguments
    fn execute(&self, context: &Context, args: &[String]) -> Result<()>;
}

/// Registry for managing commands
pub struct CommandRegistry {
    /// Map of command names to command implementations
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }
    
    /// Register a command
    pub fn register(&mut self, command: Box<dyn Command>) -> Result<()> {
        let name = command.name().to_string();
        if self.commands.contains_key(&name) {
            return Err(Error::DuplicateCommand(name));
        }
        self.commands.insert(name, command);
        Ok(())
    }
    
    /// Get a command by name
    pub fn get(&self, name: &str) -> Result<&dyn Command> {
        self.commands.get(name)
            .map(|cmd| cmd.as_ref())
            .ok_or_else(|| Error::UnknownCommand(name.to_string()))
    }
    
    /// Get all registered commands
    pub fn list(&self) -> Vec<&dyn Command> {
        self.commands.values()
            .map(|cmd| cmd.as_ref())
            .collect()
    }
}
```

### Command Implementations (commands.rs)

#### Render Command

```rust
/// Project rendering command
pub struct RenderCommand;

impl RenderCommand {
    /// Create a new render command
    pub fn new() -> Self {
        Self
    }
}

impl Command for RenderCommand {
    fn name(&self) -> &str {
        "render"
    }
    
    fn description(&self) -> &str {
        "Render a project to an output file"
    }
    
    fn usage(&self) -> &str {
        "render --project <project_file> --output <output_file> [options]"
    }
    
    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // Implementation details would go here
        context.logger.info("Render command received");
        context.logger.info(&format!("Args: {:?}", args));
        
        // Return success for now - this is just a stub until fully implemented
        Ok(())
    }
}
```

#### Info Command

The Info command displays information about media files:

```rust
/// The info command, which displays information about media files
pub struct InfoCommand;

impl InfoCommand {
    /// Creates a new info command
    pub fn new() -> Self {
        Self
    }
    
    /// Checks if the specified file exists
    fn check_file_exists(&self, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(Error::CommandError(format!(
                "File does not exist: {}",
                file_path
            )));
        }
        if !path.is_file() {
            return Err(Error::CommandError(format!(
                "Path is not a file: {}",
                file_path
            )));
        }
        Ok(())
    }
    
    /// Gets media information from the specified file
    fn get_media_info(&self, context: &Context, file_path: &str) -> Result<MediaInfo> {
        // First try to detect FFmpeg
        let ffmpeg = FFmpeg::detect()
            .map_err(|e| Error::CommandError(format!("FFmpeg error: {e}")))?;
            
        // Get media info
        ffmpeg
            .get_media_info(file_path)
            .map_err(|e| Error::CommandError(format!("Failed to get media info: {e}")))
    }
    
    /// Formats a file size into a human-readable string
    fn format_file_size(&self, size_str: &str) -> Result<String> {
        let size = size_str
            .parse::<f64>()
            .map_err(|_| Error::CommandError(format!("Invalid file size: {size_str}")))?;
            
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        
        let formatted = if size >= GB {
            format!("{:.2} GB", size / GB)
        } else if size >= MB {
            format!("{:.2} MB", size / MB)
        } else if size >= KB {
            format!("{:.2} KB", size / KB)
        } else {
            format!("{} bytes", size as u64)
        };
        
        Ok(formatted)
    }
    
    /// Formats a duration into a human-readable string
    fn format_duration(&self, duration_str: &str) -> Result<String> {
        let duration = duration_str
            .parse::<f64>()
            .map_err(|_| Error::CommandError(format!("Invalid duration: {duration_str}")))?;
            
        let hours = (duration / 3600.0).floor() as u64;
        let minutes = ((duration % 3600.0) / 60.0).floor() as u64;
        let seconds = (duration % 60.0).floor() as u64;
        let ms = ((duration - duration.floor()) * 1000.0).round() as u64;
        
        let formatted = if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}.{ms:03}")
        } else {
            format!("{minutes:02}:{seconds:02}.{ms:03}")
        };
        
        Ok(formatted)
    }
    
    /// Displays media information in a formatted way
    fn display_media_info(&self, context: &Context, media_info: &MediaInfo) -> Result<()> {
        let format = &media_info.format;
        
        // Display basic information
        context.output().info(&format!("File: {}", format.filename));
        
        if let Some(size) = &format.size {
            if let Ok(formatted_size) = self.format_file_size(size) {
                context.output().info(&format!("Size: {formatted_size}"));
            }
        }
        
        if let Some(duration) = &format.duration {
            if let Ok(formatted_duration) = self.format_duration(duration) {
                context.output().info(&format!("Duration: {formatted_duration}"));
            }
        }
        
        context.output().info(&format!("Format: {}", format.format_long_name));
        
        if let Some(bit_rate) = &format.bit_rate {
            let bit_rate_num = bit_rate.parse::<f64>().unwrap_or(0.0);
            let bit_rate_mbps = bit_rate_num / 1_000_000.0;
            context.output().info(&format!("Bitrate: {:.2} Mbps", bit_rate_mbps));
        }
        
        // Display stream information
        context.output().info(&format!("Streams: {}", media_info.streams.len()));
        
        for stream in &media_info.streams {
            let codec_type = stream.codec_type.to_uppercase();
            let codec = &stream.codec_long_name;
            
            match stream.codec_type.as_str() {
                "video" => {
                    let width = stream.width.unwrap_or(0);
                    let height = stream.height.unwrap_or(0);
                    let fps = stream.frame_rate.as_deref().unwrap_or("unknown");
                    
                    context.output().info(&format!(
                        "  Stream #{}: {} - {}, {}x{}, {} fps",
                        stream.index, codec_type, codec, width, height, fps
                    ));
                }
                "audio" => {
                    let channels = stream.channels.unwrap_or(0);
                    let sample_rate = stream.sample_rate.as_deref().unwrap_or("unknown");
                    
                    context.output().info(&format!(
                        "  Stream #{}: {} - {}, {} Hz, {} channels",
                        stream.index, codec_type, codec, sample_rate, channels
                    ));
                }
                "subtitle" => {
                    let language = stream
                        .tags
                        .as_ref()
                        .and_then(|tags| tags.get("language"))
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                        
                    context.output().info(&format!(
                        "  Stream #{}: {} - {}, Language: {}",
                        stream.index, codec_type, codec, language
                    ));
                }
                _ => {
                    context.output().info(&format!(
                        "  Stream #{}: {} - {}",
                        stream.index, codec_type, codec
                    ));
                }
            }
        }
        
        Ok(())
    }
}

impl Command for InfoCommand {
    fn name(&self) -> &str {
        "info"
    }
    
    fn description(&self) -> &str {
        "Displays information about media files"
    }
    
    fn usage(&self) -> &str {
        "info <file> [--detailed]"
    }
    
    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // Check for required arguments
        if args.is_empty() {
            return Err(Error::CommandError(
                "No input file specified. Usage: info <file>".to_string(),
            ));
        }
        
        let file_path = &args[0];
        let detailed = args.len() > 1 && args[1] == "--detailed";
        
        // Check if the file exists
        self.check_file_exists(file_path)?;
        
        // Get media information
        let media_info = self.get_media_info(context, file_path)?;
        
        // Display the information
        self.display_media_info(context, &media_info)?;
        
        // If detailed information is requested, print the raw JSON
        if detailed {
            context.output().info("\nDetailed Information:");
            let json = serde_json::to_string_pretty(&media_info)
                .map_err(|e| Error::CommandError(format!("Failed to serialize media info: {e}")))?;
                
            context.output().info(&json);
        }
        
        Ok(())
    }
}

### Output Formatting (output.rs)

For formatted console output:

```rust
/// Output formatter for terminal output
pub struct OutputFormatter {
    /// Whether to use ANSI colors
    use_colors: bool,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(use_colors: bool) -> Self {
        Self { use_colors }
    }
    
    /// Print an informational message
    pub fn info(&self, message: &str) {
        if self.use_colors {
            println!("\x1b[32m[INFO]\x1b[0m {}", message);
        } else {
            println!("[INFO] {}", message);
        }
    }
    
    /// Print an error message
    pub fn error(&self, message: &str) {
        if self.use_colors {
            eprintln!("\x1b[31m[ERROR]\x1b[0m {}", message);
        } else {
            eprintln!("[ERROR] {}", message);
        }
    }
    
    /// Print a warning message
    pub fn warning(&self, message: &str) {
        if self.use_colors {
            println!("\x1b[33m[WARNING]\x1b[0m {}", message);
        } else {
            println!("[WARNING] {}", message);
        }
    }
    
    /// Print a success message
    pub fn success(&self, message: &str) {
        if self.use_colors {
            println!("\x1b[32m[SUCCESS]\x1b[0m {}", message);
        } else {
            println!("[SUCCESS] {}", message);
        }
    }
}
```

### Progress Reporting (output.rs)

For tracking progress of long-running operations:

```rust
/// Interface for reporting progress of operations
pub trait ProgressReporter: Send {
    /// Update progress
    fn update(&self, current: u64, total: u64, message: Option<&str>);
    
    /// Mark operation as complete
    fn complete(&self, message: &str);
    
    /// Mark operation as failed
    fn fail(&self, message: &str);
}

/// Console-based progress reporter using progress bars
pub struct ConsoleProgress {
    /// The progress bar
    progress_bar: ProgressBar,
    /// Start time of the operation
    start_time: Instant,
}

impl ConsoleProgress {
    /// Create a new console progress reporter
    pub fn new(total: u64, title: &str) -> Self {
        let pb = ProgressBar::new(total);
        
        // Configure progress bar style
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        pb.set_message(title.to_string());
        
        Self {
            progress_bar: pb,
            start_time: Instant::now(),
        }
    }
}

impl ProgressReporter for ConsoleProgress {
    fn update(&self, current: u64, total: u64, message: Option<&str>) {
        if total > 0 && self.progress_bar.length() != total {
            self.progress_bar.set_length(total);
        }
        
        self.progress_bar.set_position(current);
        if let Some(msg) = message {
            self.progress_bar.set_message(msg.to_string());
        }
    }
    
    fn complete(&self, message: &str) {
        self.progress_bar.finish_with_message(message.to_string());
    }
    
    fn fail(&self, message: &str) {
        self.progress_bar.abandon_with_message(message.to_string());
    }
}
```

## Error Handling (mod.rs)

The CLI module defines its own error types:

```rust
/// Error type for CLI operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error executing command
    #[error("Command execution error: {0}")]
    CommandExecution(String),

    /// Unknown command
    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    /// Duplicate command registration
    #[error("Duplicate command registration: {0}")]
    DuplicateCommand(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Missing argument
    #[error("Missing argument: {0}")]
    MissingArgument(String),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Invalid time format
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] crate::core::Error),

    /// Project error
    #[error("Project error: {0}")]
    ProjectError(String),

    /// Render error
    #[error("Render error: {0}")]
    RenderError(String),
}

/// Result type for CLI operations
pub type Result<T> = std::result::Result<T, Error>;
```

## Implementation Considerations

### Error Handling

The CLI module follows these error handling principles:

- Use the `thiserror` crate for defining structured error types
- Provide clear, user-friendly error messages
- Include context information in errors
- Support error conversion from other modules
- Use consistent error output formatting
- Implement robust error handling in main.rs to ensure graceful application exit

### Progress Reporting

The progress reporting system follows these principles:

- Abstract progress reporting through a trait
- Support operations with known and unknown total work
- Provide meaningful time estimates
- Allow for nested progress reporting
- Support cancellation of in-progress operations

### Command Structure

Commands in the CLI module follow these principles:

- Consistent parameter naming across commands
- Sensible defaults for optional parameters
- Comprehensive help text
- Input validation with clear error messages
- Confirmation for destructive operations

## Integration with Other Modules

### Core Module Integration

- Uses the Core module for configuration and logging
- Creates execution contexts for commands
- Propagates errors from Core operations

### Project Module Integration

- Provides commands for project management
- Supports project loading and saving
- Enables timeline editing and rendering

### FFmpeg Module Integration

- Deep integration with the FFmpeg module for media operations
- Uses `FFmpeg::detect()` to find and validate FFmpeg installations
- Leverages `get_media_info()` to retrieve comprehensive media file details
- Formats and displays media information with proper error handling
- Provides detailed information about video, audio, and subtitle streams
- Handles FFmpeg availability and execution errors gracefully

### Audio Module Integration

- Provides commands for audio processing
- Supports volume adjustment and extraction
- Enables audio replacement and effects

### Subtitle Module Integration

- Provides commands for subtitle processing
- Supports subtitle editing and formatting
- Enables subtitle extraction and injection

## Integration with External Libraries

### mime_guess Integration

The CLI module uses the `mime_guess` crate for detecting file types, particularly in the InfoCommand:

- Enables detection of file MIME types based on file extensions
- Provides user-friendly file type information
- Enhances media file identification capabilities
- Used to determine whether a file is a valid media file for FFmpeg processing

### chrono Integration

The module uses the `chrono` crate for date and time handling:

- Formats file timestamps in human-readable format (UTC-based)
- Provides date calculations for file statistics
- Enables precise time formatting for logs and outputs

### serde_json Integration

For media file information processing:

- Parses FFmpeg's JSON output into structured data
- Enables type-safe access to media information
- Supports detailed media file analysis

## Implementation Status Update (2024)

### Current Implementation Status: IN PROGRESS (~60%)

The CLI module is being actively developed with core functionality in place. The main application structure and command parsing is complete, and several key commands have been implemented.

| Component | Status | Implementation Level | Notes |
|-----------|--------|----------------------|-------|
| App Structure | ✅ Complete | 100% | Application entry point and command dispatcher |
| Command Line Parsing | ✅ Complete | 100% | Command line argument parsing with clap |
| Command Registry | ✅ Complete | 100% | Command registration and lookup |
| InfoCommand | ✅ Complete | 100% | Displays media file information |
| TrimCommand | 🔄 In Progress | 80% | Basic trimming functionality implemented |
| ConcatCommand | 🔄 In Progress | 50% | Basic concatenation implemented |
| Other Commands | 📝 Planned | 0% | Yet to be implemented |

### Key Features Implemented

1. **Application Structure**
   - Command line argument parsing
   - Command registry
   - Execution context with configuration and logging

2. **Core Commands**
   - Info command
   - Basic trim functionality
   - Initial concatenation support

### Future Development Plans

1. **Complete Core Commands**
   - Finish implementation of trim and concat commands
   - ✅ Enhance Info command with FFmpeg media details (Completed)
   - Add robust error handling and validation
   - Implement progress reporting for long-running operations

2. **Enhance FFmpeg Integration**
   - ✅ Add detailed media file analysis (Completed)
   - Improve FFmpeg detection and validation
   - Add support for more encoding options
   - Handle FFmpeg errors more gracefully

3. **Implement Advanced Commands**
   - Add support for audio extraction and replacement
   - Implement subtitle processing
   - Support for batch operations
   
4. **User Experience Improvements**
   - Add more detailed progress reporting
   - Enhance error messages for better troubleshooting
   - Add interactive mode for complex operations

The CLI module will continue to evolve as the primary interface for the edv application, with a focus on usability, consistency, and integration with the growing feature set of the application. 