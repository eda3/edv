# edv - CLI Module Implementation

This document provides detailed implementation guidelines for the Command Line Interface (CLI) module of the edv application.

## Overview

The CLI module serves as the primary user interface for the edv application, handling command parsing, execution, and user interaction. It provides a consistent and intuitive command-line experience for video editing operations.

## Structure

```
src/cli/
├── mod.rs                 // Module exports
├── app.rs                 // Main application entry point
├── commands/              // Command implementations
│   ├── mod.rs             // Command registry and interfaces
│   ├── trim.rs            // Trim command implementation
│   ├── concat.rs          // Concat command implementation
│   ├── filter.rs          // Filter command implementation
│   ├── audio.rs           // Audio processing commands
│   ├── convert.rs         // Format conversion command
│   ├── subtitle.rs        // Subtitle handling commands
│   ├── project.rs         // Project management commands
│   └── batch.rs           // Batch processing commands
├── args/                  // Argument parsing
│   ├── mod.rs             // Common argument definitions
│   └── parsers.rs         // Custom argument parsers
├── output/                // Terminal output handling
│   ├── mod.rs             // Output interfaces
│   ├── formatter.rs       // Output formatting
│   └── progress.rs        // Progress bar implementation
└── utils/                 // CLI utilities
    ├── mod.rs             // Utility exports
    ├── help.rs            // Help text generation
    └── completion.rs      // Shell completion generation
```

## Key Components

### App (app.rs)

The main application entry point and command dispatcher:

```rust
pub struct App {
    config: AppConfig,
    command_registry: CommandRegistry,
    logger: Logger,
}

impl App {
    /// Create a new application instance with the given configuration
    pub fn new(config: AppConfig, logger: Logger) -> Result<Self> {
        let mut app = Self {
            config,
            command_registry: CommandRegistry::new(),
            logger,
        };
        app.register_commands()?;
        Ok(app)
    }
    
    /// Run the application with command line arguments
    pub fn run() -> Result<()> {
        let cli = Cli::parse();
        let config = AppConfig::load(cli.config.as_deref())?;
        let log_level = if cli.verbose { LogLevel::Debug } else { LogLevel::Info };
        let logger = ConsoleLogger::new(log_level);
        
        let app = Self::new(config, logger)?;
        app.execute_command(cli.command)
    }
    
    /// Register all available commands
    fn register_commands(&mut self) -> Result<()> {
        self.command_registry.register(Box::new(TrimCommand::new()))?;
        self.command_registry.register(Box::new(ConcatCommand::new()))?;
        self.command_registry.register(Box::new(FilterCommand::new()))?;
        // Register additional commands...
        Ok(())
    }
    
    /// Execute a command
    fn execute_command(&self, command: Command) -> Result<()> {
        let context = self.create_execution_context()?;
        match command {
            Command::Trim(args) => {
                let cmd = self.command_registry.get("trim")?;
                cmd.execute(&context, args)
            }
            // Handle other commands...
            _ => Err(Error::UnknownCommand(format!("{:?}", command))),
        }
    }
    
    /// Create execution context for command
    fn create_execution_context(&self) -> Result<ExecutionContext> {
        // Create execution context with necessary components
        // ...
    }
}
```

### Command Interface (commands/mod.rs)

The trait defining the interface for all commands:

```rust
/// Trait for implementing commands
pub trait Command {
    /// Get the name of the command
    fn name(&self) -> &str;
    
    /// Get the description of the command
    fn description(&self) -> &str;
    
    /// Get usage examples
    fn usage(&self) -> &str;
    
    /// Build command arguments
    fn configure_args(&self, app: Command) -> Command;
    
    /// Execute the command with given context and arguments
    fn execute(&self, context: &ExecutionContext, args: &ArgMatches) -> Result<()>;
}

/// Registry for managing commands
pub struct CommandRegistry {
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

### Command Implementations

Each command implements the Command trait:

```rust
// Trim command implementation (commands/trim.rs)
pub struct TrimCommand;

impl TrimCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Command for TrimCommand {
    fn name(&self) -> &str {
        "trim"
    }
    
    fn description(&self) -> &str {
        "Trim a video file to a specified duration"
    }
    
    fn usage(&self) -> &str {
        "edv trim --input input.mp4 --output output.mp4 --start 00:00:10 --end 00:01:00"
    }
    
    fn configure_args(&self, app: Command) -> Command {
        app.arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Input video file")
                .required(true)
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output video file")
                .required(true)
        )
        .arg(
            Arg::new("start")
                .short('s')
                .long("start")
                .value_name("TIME")
                .help("Start time (format: HH:MM:SS.mmm or seconds)")
                .required(false)
        )
        .arg(
            Arg::new("end")
                .short('e')
                .long("end")
                .value_name("TIME")
                .help("End time (format: HH:MM:SS.mmm or seconds)")
                .required(false)
        )
        .arg(
            Arg::new("recompress")
                .short('r')
                .long("recompress")
                .help("Recompress the video instead of stream copying")
                .action(ArgAction::SetTrue)
        )
    }
    
    fn execute(&self, context: &ExecutionContext, args: &ArgMatches) -> Result<()> {
        // Extract arguments
        let input = args.get_one::<String>("input").unwrap();
        let output = args.get_one::<String>("output").unwrap();
        let start = args.get_one::<String>("start").map(|s| {
            TimePosition::from_string(s).unwrap_or_else(|_| {
                context.logger.warning(&format!("Invalid start time: {}, using 0", s));
                TimePosition::from_seconds(0.0)
            })
        });
        let end = args.get_one::<String>("end").map(|e| {
            TimePosition::from_string(e).unwrap_or_else(|_| {
                context.logger.warning(&format!("Invalid end time: {}, using file end", e));
                TimePosition::from_seconds(f64::MAX)
            })
        });
        let recompress = args.get_flag("recompress");
        
        // Create operation
        let operation = TrimOperation::new(
            &Path::new(input),
            &Path::new(output),
            start,
            end,
            recompress,
        );
        
        // Setup progress reporter
        let progress = context.create_progress_bar(
            "Trimming video",
            None, // We'll get the duration from the file
        );
        
        // Execute operation
        context.get_pipeline().execute(Box::new(operation), Some(progress))
    }
}
```

### Progress Display (output/progress.rs)

```rust
pub struct ProgressDisplay {
    progress_bar: ProgressBar,
    format: ProgressFormat,
    total_steps: u64,
    start_time: Instant,
}

impl ProgressDisplay {
    /// Create a new progress display with the given total steps and format
    pub fn new(total_steps: u64, format: ProgressFormat) -> Self {
        let pb = ProgressBar::new(total_steps);
        // Set style based on format
        match format {
            ProgressFormat::Bytes => {
                pb.set_style(ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})"
                ).unwrap());
            }
            ProgressFormat::Percentage => {
                pb.set_style(ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% ({eta})"
                ).unwrap());
            }
            ProgressFormat::Time => {
                pb.set_style(ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({eta})"
                ).unwrap());
            }
        }
        
        Self {
            progress_bar: pb,
            format,
            total_steps,
            start_time: Instant::now(),
        }
    }
    
    /// Update progress with the current position and optional message
    pub fn update(&self, progress: u64, message: Option<String>) {
        self.progress_bar.set_position(progress);
        if let Some(msg) = message {
            self.progress_bar.set_message(msg);
        }
    }
    
    /// Mark progress as finished with a completion message
    pub fn finish(&self, message: String) {
        self.progress_bar.finish_with_message(message);
    }
    
    /// Mark progress as failed with an error message
    pub fn error(&self, message: String) {
        self.progress_bar.abandon_with_message(message);
    }
    
    /// Get the elapsed time since progress started
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}
```

## Implementation Details

### Command Line Parsing

The edv CLI uses clap for command line parsing:

```rust
// Main command structure
#[derive(Parser)]
#[clap(name = "edv", about = "Video editing tool")]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(author = env!("CARGO_PKG_AUTHORS"))]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
    
    #[clap(short, long, global = true)]
    pub verbose: bool,
    
    #[clap(short, long, global = true)]
    pub config: Option<PathBuf>,
}

// Subcommand enum
#[derive(Subcommand)]
pub enum Command {
    #[clap(about = "Trim a video")]
    Trim(TrimArgs),
    
    #[clap(about = "Concatenate videos")]
    Concat(ConcatArgs),
    
    #[clap(about = "Apply filters to a video")]
    Filter(FilterArgs),
    
    #[clap(about = "Process audio in a video")]
    Audio(AudioArgs),
    
    #[clap(about = "Convert video format")]
    Convert(ConvertArgs),
    
    #[clap(about = "Work with subtitles")]
    Subtitle(SubtitleArgs),
    
    #[clap(about = "Manage projects")]
    Project(ProjectArgs),
    
    #[clap(about = "Process multiple files")]
    Batch(BatchArgs),
}
```

### Custom Argument Parsing

For specialized types like time positions:

```rust
// Time position argument parser
pub struct TimePositionValueParser;

impl ValueParser for TimePositionValueParser {
    type Value = TimePosition;

    fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
        let value_str = value.to_str().ok_or_else(|| {
            Error::new(ErrorKind::InvalidUtf8).with_cmd(cmd).with_arg(arg)
        })?;
        
        TimePosition::from_string(value_str).map_err(|e| {
            Error::new(ErrorKind::InvalidValue)
                .with_cmd(cmd)
                .with_arg(arg)
                .with_message(format!("Invalid time format: {}", e))
        })
    }
}
```

### Terminal Output

For formatted console output:

```rust
pub enum OutputFormat {
    Plain,
    Colored,
    Json,
}

pub struct OutputFormatter {
    format: OutputFormat,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }
    
    pub fn print_info(&self, message: &str) {
        match self.format {
            OutputFormat::Plain => println!("INFO: {}", message),
            OutputFormat::Colored => println!("{} {}", "INFO:".blue(), message),
            OutputFormat::Json => println!("{{\"level\":\"info\",\"message\":{}}}", 
                                          serde_json::to_string(message).unwrap()),
        }
    }
    
    pub fn print_error(&self, message: &str) {
        match self.format {
            OutputFormat::Plain => eprintln!("ERROR: {}", message),
            OutputFormat::Colored => eprintln!("{} {}", "ERROR:".red().bold(), message),
            OutputFormat::Json => println!("{{\"level\":\"error\",\"message\":{}}}", 
                                          serde_json::to_string(message).unwrap()),
        }
    }
    
    pub fn print_warning(&self, message: &str) {
        match self.format {
            OutputFormat::Plain => println!("WARNING: {}", message),
            OutputFormat::Colored => println!("{} {}", "WARNING:".yellow(), message),
            OutputFormat::Json => println!("{{\"level\":\"warning\",\"message\":{}}}", 
                                          serde_json::to_string(message).unwrap()),
        }
    }
    
    pub fn print_success(&self, message: &str) {
        match self.format {
            OutputFormat::Plain => println!("SUCCESS: {}", message),
            OutputFormat::Colored => println!("{} {}", "SUCCESS:".green(), message),
            OutputFormat::Json => println!("{{\"level\":\"success\",\"message\":{}}}", 
                                          serde_json::to_string(message).unwrap()),
        }
    }
}
```

## Implementation Considerations

### Error Handling

- Provide clear, user-friendly error messages
- Include context information in errors
- Suggest remediation steps when appropriate
- Use consistent error output formatting

```rust
fn handle_error(error: Error, formatter: &OutputFormatter) -> i32 {
    match error {
        Error::InvalidArgument(msg) => {
            formatter.print_error(&format!("Invalid argument: {}", msg));
            formatter.print_info("Try 'edv --help' for more information.");
            1
        }
        Error::FileNotFound(path) => {
            formatter.print_error(&format!("File not found: {}", path.display()));
            2
        }
        Error::FFmpegError(msg) => {
            formatter.print_error(&format!("FFmpeg error: {}", msg));
            3
        }
        // Handle other error types...
        _ => {
            formatter.print_error(&format!("Unexpected error: {}", error));
            99
        }
    }
}
```

### Signal Handling

Handle keyboard interrupts and other signals gracefully:

```rust
fn setup_signal_handlers() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nReceived Ctrl+C, shutting down gracefully...");
    })?;
    
    Ok(())
}
```

### Progress Reporting

Implement robust progress reporting for long-running operations:

- Parse FFmpeg output to extract progress information
- Update progress bar based on elapsed time or frame count
- Provide estimated time remaining
- Handle streaming operations without known duration

### Shell Completion

Generate shell completion scripts for various shells:

```rust
pub fn generate_completions(shell: Shell, out_dir: &Path) -> Result<()> {
    let mut app = Cli::command();
    let name = app.get_name().to_string();
    
    generate_to(shell, &mut app, name, out_dir)?;
    Ok(())
}
```

## Integration with Other Modules

### Core Module Integration

- Use the Core module for configuration access
- Create execution context for commands
- Access centralized error handling

### Processing Module Integration

- Execute operations via the processing pipeline
- Track progress of operations
- Handle operation failures

### Project Module Integration

- Access project state for project-related commands
- Save and load projects
- Validate project operations

## Testing Strategy

### Unit Testing

Test individual components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_registration() {
        let mut registry = CommandRegistry::new();
        let command = Box::new(TrimCommand::new());
        assert!(registry.register(command).is_ok());
        
        // Test duplicate registration fails
        let command2 = Box::new(TrimCommand::new());
        assert!(registry.register(command2).is_err());
    }
    
    #[test]
    fn test_command_lookup() {
        let mut registry = CommandRegistry::new();
        registry.register(Box::new(TrimCommand::new())).unwrap();
        
        let cmd = registry.get("trim");
        assert!(cmd.is_ok());
        assert_eq!(cmd.unwrap().name(), "trim");
        
        let not_found = registry.get("nonexistent");
        assert!(not_found.is_err());
    }
}
```

### Integration Testing

Test command execution end-to-end:

```rust
#[test]
fn test_trim_command_execution() {
    // Create test files
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = copy_fixture_to_temp("test_fixtures/sample.mp4", &temp_dir);
    let output_path = temp_dir.path().join("output.mp4");
    
    // Create config and context
    let config = AppConfig::load_default().unwrap();
    let logger = MockLogger::new();
    let context = ExecutionContext::new(config, Box::new(logger)).unwrap();
    
    // Create command and args
    let command = TrimCommand::new();
    let matches = Cli::command()
        .get_matches_from(vec![
            "edv", "trim",
            "--input", input_path.to_str().unwrap(),
            "--output", output_path.to_str().unwrap(),
            "--start", "00:00:01",
            "--end", "00:00:05",
        ]);
    
    // Extract the trim subcommand matches
    let sub_matches = matches.subcommand_matches("trim").unwrap();
    
    // Execute command
    let result = command.execute(&context, sub_matches);
    assert!(result.is_ok());
    
    // Verify output file exists and has correct duration
    assert!(output_path.exists());
    // Verify duration is approximately 4 seconds (5s - 1s)
    // ...
}
```

This detailed module implementation guide provides a comprehensive blueprint for implementing the CLI module of the edv application, covering structure, key components, implementation details, and testing strategy. 