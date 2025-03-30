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
        self.command_registry
            .register(Box::new(commands::RenderCommand::new()))?;
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
                // Info implementation
            },
            // Other commands...
        }
        
        Ok(())
    }
    
    /// Creates an execution context for command execution
    fn create_execution_context(&self) -> Result<Context> {
        Ok(Context::new(self.config.clone(), Arc::new(self.logger.clone())))
    }
}

/// Application entry point
pub fn run() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Configure logger based on verbosity
    let log_level = if cli.verbose { LogLevel::Debug } else { LogLevel::Info };
    let logger = Box::new(ConsoleLogger::new(log_level, true));
    
    // Create config (could load from file if specified in cli.config)
    let config = Config::default();
    
    // Create and initialize application
    let mut app = App::new(config, logger);
    app.initialize()?;
    
    // Execute command
    app.execute_command(cli.command)
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

Example of a command implementation:

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
    
    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // Implementation details would go here
        // For example:
        // - Parse arguments specific to rendering
        // - Load the project
        // - Configure rendering options
        // - Execute the render
        // - Report progress
        
        context.logger.info("Project rendered successfully");
        Ok(())
    }
}
```

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

### Audio Module Integration

- Provides commands for audio processing
- Supports volume adjustment and extraction
- Enables audio replacement and effects

### Subtitle Module Integration

- Provides commands for subtitle processing
- Supports subtitle editing and formatting
- Enables subtitle extraction and injection

## Implementation Status Update (2024)

### Current Implementation Status

The CLI module is in active development with the following status:

| Component | Status | Notes |
|-----------|--------|-------|
| Application Structure | ✅ Complete | Core application framework implemented |
| Command Registry | ✅ Complete | Dynamic command registration and discovery |
| Command Parsing | ✅ Complete | Argument parsing with clap |
| Output Formatting | ✅ Complete | Terminal output with color support |
| Progress Reporting | ✅ Complete | Progress bar implementation |
| Error Handling | ✅ Complete | Comprehensive error types and messages |
| Basic Commands | 🔄 In Progress | Core video editing commands partially implemented |
| Project Commands | 🔄 In Progress | Render command implemented, others in development |
| Audio Commands | 🔶 Planned | Design completed, implementation coming soon |
| Subtitle Commands | 🔶 Planned | Design completed, implementation coming soon |

### Future Development Plans

The following enhancements are planned for the CLI module:

1. **Complete Core Commands**
   - Finish implementation of Trim, Concat, and Info commands
   - Add Convert command for format conversion
   - Implement Extract command for stream extraction

2. **Audio and Subtitle Commands**
   - Implement Volume command for audio volume adjustment
   - Add SubtitleEdit command for subtitle editing
   - Develop SubtitleSync command for subtitle synchronization

3. **Enhanced Project Management**
   - Add ProjectCreate and ProjectOpen commands
   - Implement Timeline editing commands
   - Develop Asset management commands

4. **User Experience Improvements**
   - Add shell completion support
   - Enhance error messaging with suggestions
   - Implement verbose logging options
   - Add dry-run mode for command testing

The CLI module will continue to evolve as the primary interface for the edv application, with a focus on usability, consistency, and integration with the growing feature set of the application. 