# edv - Core Module Implementation

This document provides detailed implementation guidelines for the Core module of the edv application, which provides central functionality used by other modules.

## Overview

The Core module serves as the foundation for the entire application, providing essential services and abstractions that other modules depend on. It handles configuration management, logging, error handling, and execution context.

## Structure

```
src/core/
├── mod.rs                 // Module exports
├── config.rs              // Configuration management
├── context.rs             // Execution context
├── error.rs               // Error handling
├── logger/                // Logging system
│   ├── mod.rs             // Logger exports
│   ├── console.rs         // Console logger implementation
│   └── file.rs            // File logger implementation
└── utils/                 // Core utilities
    ├── mod.rs             // Utility exports
    └── version.rs         // Version information
```

## Key Components

### Configuration Manager (config.rs)

The configuration manager handles application-wide settings:

```rust
pub struct AppConfig {
    pub ffmpeg_path: PathBuf,
    pub temp_directory: PathBuf,
    pub default_format: String,
    pub threading: ThreadingConfig,
    pub logging: LoggingConfig,
}

impl AppConfig {
    /// Load configuration from the default location
    pub fn load_default() -> Result<Self> {
        Self::load(None)
    }
    
    /// Load configuration from a specified path
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let mut config_manager = ConfigManager::new();
        
        // Add sources in priority order
        if let Some(config_path) = path {
            config_manager.add_source(ConfigSource::File(config_path.to_path_buf()));
        }
        config_manager.add_source(ConfigSource::Environment);
        config_manager.add_source(ConfigSource::Defaults);
        
        config_manager.load()?;
        Ok(config_manager.get_config().clone())
    }
    
    /// Save configuration to a specified path
    pub fn save(&self, path: &Path) -> Result<()> {
        let config_json = serde_json::to_string_pretty(self)?;
        fs::write(path, config_json)?;
        Ok(())
    }
}

/// Configuration source enum
pub enum ConfigSource {
    File(PathBuf),
    Environment,
    Defaults,
}

/// Threading configuration
pub struct ThreadingConfig {
    pub thread_count: usize,
    pub priority: ThreadPriority,
}

/// Logging configuration
pub struct LoggingConfig {
    pub level: LogLevel,
    pub destinations: Vec<LogDestination>,
    pub format: LogFormat,
}

/// Configuration manager
pub struct ConfigManager {
    sources: Vec<ConfigSource>,
    current_config: AppConfig,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            current_config: AppConfig::default(),
        }
    }
    
    /// Add a configuration source
    pub fn add_source(&mut self, source: ConfigSource) {
        self.sources.push(source);
    }
    
    /// Load configuration from all sources
    pub fn load(&mut self) -> Result<()> {
        // Start with defaults
        self.current_config = AppConfig::default();
        
        // Apply sources in order
        for source in &self.sources {
            match source {
                ConfigSource::File(path) => {
                    if path.exists() {
                        let file_content = fs::read_to_string(path)?;
                        let file_config: AppConfig = serde_json::from_str(&file_content)?;
                        self.merge_config(file_config);
                    }
                }
                ConfigSource::Environment => {
                    self.load_from_environment();
                }
                ConfigSource::Defaults => {
                    // Already loaded defaults
                }
            }
        }
        
        // Validate the final configuration
        self.validate_config()?;
        
        Ok(())
    }
    
    /// Get the current configuration
    pub fn get_config(&self) -> &AppConfig {
        &self.current_config
    }
    
    /// Merge a configuration into the current one
    fn merge_config(&mut self, other: AppConfig) {
        // Implementation of config merging logic
    }
    
    /// Load configuration from environment variables
    fn load_from_environment(&mut self) {
        // Implementation of environment loading
    }
    
    /// Validate the current configuration
    fn validate_config(&self) -> Result<()> {
        // Implementation of validation logic
        Ok(())
    }
}
```

### Execution Context (context.rs)

The execution context provides a container for shared resources:

```rust
pub struct Context {
    pub config: AppConfig,
    pub logger: Box<dyn Logger>,
    pub progress_reporter: Option<Box<dyn ProgressReporter>>,
    temp_dir: Option<TempDir>,
}

impl Context {
    /// Create a new execution context
    pub fn new(config: AppConfig, logger: Box<dyn Logger>) -> Result<Self> {
        Ok(Self {
            config,
            logger,
            progress_reporter: None,
            temp_dir: None,
        })
    }
    
    /// Set the progress reporter
    pub fn with_progress_reporter(mut self, reporter: Box<dyn ProgressReporter>) -> Self {
        self.progress_reporter = Some(reporter);
        self
    }
    
    /// Get a reference to the processing pipeline
    pub fn get_pipeline(&self) -> Result<ProcessingPipeline> {
        ProcessingPipeline::new(self.config.clone())
    }
    
    /// Create a temporary directory
    pub fn create_temp_dir(&mut self) -> Result<&Path> {
        if self.temp_dir.is_none() {
            let temp_dir = TempDir::new_in(&self.config.temp_directory, "edv")?;
            let path = temp_dir.path().to_path_buf();
            self.temp_dir = Some(temp_dir);
            Ok(path.as_path())
        } else {
            Ok(self.temp_dir.as_ref().unwrap().path())
        }
    }
    
    /// Create a progress bar
    pub fn create_progress_bar(&self, title: &str, total: Option<u64>) -> Box<dyn ProgressReporter> {
        if let Some(reporter) = &self.progress_reporter {
            reporter.create_progress_bar(title, total)
        } else {
            Box::new(NullProgressReporter)
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Clean up resources
        if let Some(temp_dir) = self.temp_dir.take() {
            let _ = temp_dir.close();
        }
    }
}
```

### Error Handling (error.rs)

Centralized error handling for the application:

```rust
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub trait ErrorContextExt<T> {
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: Display + Send + Sync + 'static;
}

impl<T> ErrorContextExt<T> for Result<T> {
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|err| {
            let context_str = context().to_string();
            match err {
                Error::Io(io_err) => Error::Io(io::Error::new(
                    io_err.kind(),
                    format!("{}: {}", context_str, io_err),
                )),
                Error::FFmpeg(msg) => Error::FFmpeg(format!("{}: {}", context_str, msg)),
                Error::InvalidArgument(msg) => Error::InvalidArgument(format!("{}: {}", context_str, msg)),
                Error::OperationFailed(msg) => Error::OperationFailed(format!("{}: {}", context_str, msg)),
                Error::Configuration(msg) => Error::Configuration(format!("{}: {}", context_str, msg)),
                Error::Serialization(err) => Error::Serialization(err),
                Error::NotFound(msg) => Error::NotFound(format!("{}: {}", context_str, msg)),
                Error::Validation(msg) => Error::Validation(format!("{}: {}", context_str, msg)),
                Error::Internal(msg) => Error::Internal(format!("{}: {}", context_str, msg)),
            }
        })
    }
}
```

### Logging System (logger/)

The logging system manages application-wide logging:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

pub trait Logger: Send + Sync {
    fn log(&self, level: LogLevel, message: &str);
    
    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
    
    fn warning(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }
    
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }
    
    fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
    
    fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }
}

#[derive(Debug, Clone)]
pub enum LogDestination {
    Console,
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub enum LogFormat {
    Plain,
    Structured,
}

pub struct ConsoleLogger {
    level: LogLevel,
}

impl ConsoleLogger {
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.level {
            match level {
                LogLevel::Error => eprintln!("ERROR: {}", message),
                LogLevel::Warning => println!("WARNING: {}", message),
                LogLevel::Info => println!("INFO: {}", message),
                LogLevel::Debug => println!("DEBUG: {}", message),
                LogLevel::Trace => println!("TRACE: {}", message),
            }
        }
    }
}

pub struct FileLogger {
    level: LogLevel,
    file: Mutex<File>,
}

impl FileLogger {
    pub fn new(path: &Path, level: LogLevel) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            level,
            file: Mutex::new(file),
        })
    }
}

impl Logger for FileLogger {
    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.level {
            if let Ok(mut file) = self.file.lock() {
                let _ = writeln!(file, "{}: {}", level, message);
            }
        }
    }
}

pub struct MultiLogger {
    loggers: Vec<Box<dyn Logger>>,
}

impl MultiLogger {
    pub fn new(loggers: Vec<Box<dyn Logger>>) -> Self {
        Self { loggers }
    }
}

impl Logger for MultiLogger {
    fn log(&self, level: LogLevel, message: &str) {
        for logger in &self.loggers {
            logger.log(level, message);
        }
    }
}
```

## Implementation Guidelines

When implementing the Core module, follow these guidelines:

1. **Configuration System**:
   - Use a layered approach to configuration (defaults, file, environment)
   - Support partial configuration updates
   - Validate configuration values

2. **Error Handling**:
   - Create a comprehensive error enum
   - Support error chaining
   - Include context information in errors

3. **Logging System**:
   - Support different log levels
   - Allow multiple log destinations
   - Include timestamps and context in log messages

4. **Context Management**:
   - Provide access to shared services
   - Manage resource cleanup
   - Support context cloning or referencing

## Best Practices for Ownership and Borrowing

When designing and implementing the Core module components, follow these best practices for Rust's ownership and borrowing system:

### 1. Prefer Borrowing Over Ownership Transfer

```rust
// Good: Using borrowed references
pub fn process_config(&self, config: &AppConfig) -> Result<()> {
    // Work with config by reference
}

// Avoid: Taking ownership unnecessarily
pub fn process_config(&self, config: AppConfig) -> Result<()> {
    // Now we own config, which might not be necessary
}
```

### 2. Return Owned Data for Factory Methods

```rust
// Good: Return owned data from constructors/factories
pub fn create_default_config() -> AppConfig {
    AppConfig {
        ffmpeg_path: default_ffmpeg_path(),
        temp_directory: default_temp_dir(),
        // Other defaults...
    }
}
```

### 3. Use Clone When Shared Ownership is Needed

```rust
// Good: Clone when you need shared ownership
pub fn with_modified_config(&self, modify_fn: impl FnOnce(&mut AppConfig)) -> Self {
    let mut config = self.config.clone();
    modify_fn(&mut config);
    Self {
        config,
        logger: self.logger.clone(),
        // Other fields...
    }
}
```

### 4. Use Accessor Methods for Immutable and Mutable Access

```rust
// Good: Provide both immutable and mutable accessors
impl Context {
    // Immutable access
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
    
    // Mutable access when needed
    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }
}
```

### 5. Implement Copy for Small Types

```rust
// Good: Implement Copy for small, stack-allocated types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}
```

### 6. Use Arc for Shared References in Multithreaded Code

```rust
// Good: Use Arc for thread-safe shared ownership
use std::sync::Arc;

pub struct WorkerPool {
    shared_config: Arc<AppConfig>,
    // Other fields...
}

impl WorkerPool {
    pub fn new(config: AppConfig) -> Self {
        Self {
            shared_config: Arc::new(config),
            // Initialize other fields...
        }
    }
    
    pub fn spawn_worker(&self) -> Worker {
        Worker::new(Arc::clone(&self.shared_config))
    }
}
```

### 7. Avoid Self-Referential Structures

```rust
// Avoid: Self-referential structures are difficult in Rust
pub struct BadContext {
    config: AppConfig,
    // This will cause ownership problems
    config_ref: &AppConfig, // Error: reference to field inside same struct
}

// Good: Use indices or IDs instead
pub struct GoodContext {
    configs: Vec<AppConfig>,
    active_config_index: usize,
}
```

### 8. Design APIs for Ergonomic Use

```rust
// Good: Design APIs to be both safe and ergonomic
impl Logger {
    // Chainable, takes &mut self
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
    
    // Takes &self since it doesn't need to modify
    pub fn log(&self, level: LogLevel, message: &str) {
        if level <= self.level {
            // Log the message
        }
    }
}
```

By following these best practices, the Core module will provide a solid foundation for the rest of the application while adhering to Rust's safety principles.

## Example Implementation

### Configuration Loading

The configuration is loaded in a cascading manner, with sources having different priorities:

1. Command-line arguments (highest priority)
2. Configuration file specified by command-line
3. Environment variables
4. Default configuration (lowest priority)

```rust
fn load_configuration() -> Result<AppConfig> {
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = AppConfig::load(cli.config.as_deref())?;
    
    // Override with command-line options
    let mut config = config.clone();
    if cli.verbose {
        config.logging.level = LogLevel::Debug;
    }
    
    Ok(config)
}
```

### Version Information

Version information is managed in a centralized module:

```rust
pub struct VersionInfo {
    pub version: String,
    pub build_date: String,
    pub git_hash: String,
    pub rust_version: String,
}

impl VersionInfo {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_date: env!("BUILD_DATE").to_string(),
            git_hash: env!("GIT_HASH").to_string(),
            rust_version: env!("RUST_VERSION").to_string(),
        }
    }
    
    pub fn full_version_string(&self) -> String {
        format!(
            "edv v{} (built on {}, commit {}, using Rust {})",
            self.version, self.build_date, self.git_hash, self.rust_version
        )
    }
}
```

### Environment Variables

The application uses environment variables for configuration:

```rust
fn load_from_environment(&mut self) {
    if let Ok(ffmpeg_path) = env::var("EDV_FFMPEG_PATH") {
        self.current_config.ffmpeg_path = PathBuf::from(ffmpeg_path);
    }
    
    if let Ok(temp_dir) = env::var("EDV_TEMP_DIR") {
        self.current_config.temp_directory = PathBuf::from(temp_dir);
    }
    
    if let Ok(log_level) = env::var("EDV_LOG_LEVEL") {
        self.current_config.logging.level = match log_level.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warning" => LogLevel::Warning,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => self.current_config.logging.level,
        };
    }
    
    if let Ok(thread_count) = env::var("EDV_THREAD_COUNT") {
        if let Ok(count) = thread_count.parse::<usize>() {
            self.current_config.threading.thread_count = count;
        }
    }
}
```

### FFmpeg Detection

The core module handles automatic detection of FFmpeg:

```rust
fn detect_ffmpeg() -> Result<PathBuf> {
    // Check common installation locations
    let locations = if cfg!(target_os = "windows") {
        vec![
            PathBuf::from("C:\\Program Files\\FFmpeg\\bin\\ffmpeg.exe"),
            PathBuf::from("C:\\Program Files (x86)\\FFmpeg\\bin\\ffmpeg.exe"),
        ]
    } else {
        vec![
            PathBuf::from("/usr/bin/ffmpeg"),
            PathBuf::from("/usr/local/bin/ffmpeg"),
            PathBuf::from("/opt/local/bin/ffmpeg"),
        ]
    };
    
    // Check if FFmpeg exists in any of the locations
    for location in locations {
        if location.exists() {
            return Ok(location);
        }
    }
    
    // Check system PATH
    if let Ok(output) = Command::new("which").arg("ffmpeg").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            return Ok(path);
        }
    }
    
    Err(Error::NotFound("FFmpeg not found in system".to_string()))
}
```

## Integration with Other Modules

### CLI Module Integration

The Core module provides the execution context and configuration for the CLI module:

```rust
pub fn execute_command(command: Command, config: AppConfig) -> Result<()> {
    // Create logger
    let log_level = config.logging.level;
    let logger: Box<dyn Logger> = Box::new(ConsoleLogger::new(log_level));
    
    // Create context
    let context = Context::new(config, logger)?;
    
    // Execute command
    match command {
        Command::Trim(args) => execute_trim(args, &context),
        Command::Concat(args) => execute_concat(args, &context),
        Command::Info(args) => execute_info(args, &context),
        // Other commands...
    }
}
```

### Processing Module Integration

The Core module provides the configuration and context for the Processing module:

```rust
pub fn get_processing_pipeline(&self) -> Result<ProcessingPipeline> {
    ProcessingPipeline::new(self.config.clone())
}
```

### Project Module Integration

The Core module provides error handling and context for the Project module:

```rust
pub fn execute_project_command(args: &ProjectArgs, context: &Context) -> Result<()> {
    match &args.subcommand {
        ProjectSubcommand::New(new_args) => {
            let project = Project::new(&new_args.name);
            let project_manager = ProjectManager::new(context.config.clone())?;
            project_manager.save_project(&project, &PathBuf::from(&new_args.path))?;
            context.logger.info(&format!("Created new project at {}", new_args.path));
            Ok(())
        }
        ProjectSubcommand::Open(open_args) => {
            let project_manager = ProjectManager::new(context.config.clone())?;
            let project = project_manager.load_project(&PathBuf::from(&open_args.path))?;
            context.logger.info(&format!("Opened project '{}'", project.metadata.name));
            // Further project operations...
            Ok(())
        }
        // Other subcommands...
    }
}
```

## Testing Strategy

### Unit Testing

Test individual components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_load_default() {
        let config = AppConfig::default();
        assert_eq!(config.logging.level, LogLevel::Info);
        
        // Test defaults match expected values
        assert_eq!(config.threading.thread_count, num_cpus::get());
    }
    
    #[test]
    fn test_config_merge() {
        let mut config_manager = ConfigManager::new();
        
        // Create a base config
        let mut base_config = AppConfig::default();
        base_config.logging.level = LogLevel::Error;
        
        // Create an override config
        let mut override_config = AppConfig::default();
        override_config.logging.level = LogLevel::Debug;
        
        // Set the base and merge
        config_manager.current_config = base_config;
        config_manager.merge_config(override_config);
        
        // Verify the override took effect
        assert_eq!(config_manager.current_config.logging.level, LogLevel::Debug);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config_manager = ConfigManager::new();
        
        // Create an invalid config
        let mut invalid_config = AppConfig::default();
        invalid_config.threading.thread_count = 0;
        
        // Set the config and validate
        config_manager.current_config = invalid_config;
        assert!(config_manager.validate_config().is_err());
    }
}
```

### Integration Testing

Test the interaction between Core and other modules:

```rust
#[test]
fn test_context_creation_and_usage() {
    // Create a configuration
    let config = AppConfig::default();
    
    // Create a mock logger
    let mock_logger = Box::new(MockLogger::new());
    
    // Create context
    let mut context = Context::new(config, mock_logger).unwrap();
    
    // Test temp directory creation
    let temp_dir = context.create_temp_dir().unwrap();
    assert!(temp_dir.exists());
    
    // Test progress bar creation
    let progress = context.create_progress_bar("Test Progress", Some(100));
    progress.update(50, None);
    
    // Context should clean up temp dir when dropped
    drop(context);
    assert!(!temp_dir.exists());
}
```

This detailed module implementation guide provides a comprehensive blueprint for implementing the Core module of the edv application, covering structure, key components, implementation details, and testing strategy. 