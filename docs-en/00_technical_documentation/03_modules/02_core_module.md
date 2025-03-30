# edv - Core Module Implementation

This document provides detailed implementation guidelines for the Core module of the edv application.

## Overview

The Core module provides fundamental services and structures used throughout the application, including:

- Configuration management (`Config` struct)
- Execution context (`Context` struct)
- Logging system (`Logger` trait and implementations)
- Error handling (`Error` enum)

The Core module is critical for establishing a consistent foundation for other modules, enabling them to focus on their specialized responsibilities without reinventing common infrastructure.

## Structure

The Core module is organized as follows:

```
src/core/
├── mod.rs       # Module exports, Config, Context, Error, LogLevel, Logger trait
└── console.rs   # ConsoleLogger implementation
```

## Key Components

### Configuration Management (`mod.rs`)

The configuration system manages application settings:

```rust
/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Application version
    pub version: String,
    /// Verbosity level (0-4)
    pub verbosity: u8,
    /// Whether to use colored output
    pub use_colors: bool,
    /// FFmpeg executable path (if None, searches in PATH)
    pub ffmpeg_path: Option<String>,
    /// FFprobe executable path (if None, searches in PATH)
    pub ffprobe_path: Option<String>,
    /// Temporary directory (if None, uses system temp)
    pub temp_directory: Option<PathBuf>,
}

impl Config {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            verbosity: 2, // Default to INFO level
            use_colors: true,
            ffmpeg_path: None,
            ffprobe_path: None,
            temp_directory: None,
        }
    }

    /// Returns the log level based on verbosity
    pub fn log_level(&self) -> LogLevel {
        match self.verbosity {
            0 => LogLevel::Error,
            1 => LogLevel::Warning,
            2 => LogLevel::Info,
            3 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
```

**Responsibilities:**
- Stores application-wide configuration settings
- Maps verbosity levels to log levels
- Provides defaults for all configuration options
- Maintains FFmpeg/FFprobe paths and temporary directory settings

**Implementation Notes:**
- The `Config` struct is simple and focused on essential settings.
- It does not currently include serialization/deserialization capabilities.
- Future enhancements may include loading from file/environment.

### Execution Context (`mod.rs`)

The context provides a shared environment for operations:

```rust
/// Execution context for operations
#[derive(Clone)]
pub struct Context {
    /// Application configuration
    pub config: Config,
    /// Logger implementation
    pub logger: Arc<dyn Logger>,
}

impl Context {
    /// Creates a new context with the given configuration and logger
    pub fn new(config: Config, logger: Arc<dyn Logger>) -> Self {
        Self { config, logger }
    }
    
    /// Creates a new context with default configuration and console logger
    pub fn default() -> Self {
        let config = Config::default();
        let logger = Arc::new(ConsoleLogger::new(config.log_level(), config.use_colors));
        Self { config, logger }
    }
    
    /// Creates a child context with the same configuration but a potentially different logger
    pub fn with_logger(&self, logger: Arc<dyn Logger>) -> Self {
        Self {
            config: self.config.clone(),
            logger,
        }
    }
}
```

**Responsibilities:**
- Provides a unified context for all operations
- Combines configuration and logging in a single container
- Enables context sharing and derivation

**Implementation Notes:**
- The `Context` is kept minimal with only essential components.
- Uses `Arc` to allow sharing the logger across threads.
- Does not maintain state beyond configuration and logging.

### Logging System (`mod.rs` and `console.rs`)

The logging system provides a common interface for application logging:

```rust
/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

/// Logger trait that must be implemented by all loggers
pub trait Logger: Send + Sync {
    /// Logs a message at the specified level
    fn log(&self, level: LogLevel, message: &str);
    
    /// Logs an error message
    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
    
    /// Logs a warning message
    fn warning(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }
    
    /// Logs an info message
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }
    
    /// Logs a debug message
    fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
    
    /// Logs a trace message
    fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }
}
```

**Console Logger Implementation (`console.rs`):**

```rust
/// Logger that outputs to the console
pub struct ConsoleLogger {
    /// Minimum level to log
    level: LogLevel,
    /// Whether to use colors
    use_colors: bool,
}

impl ConsoleLogger {
    /// Creates a new console logger with the specified level
    pub fn new(level: LogLevel, use_colors: bool) -> Self {
        Self { level, use_colors }
    }
    
    /// Gets the color for a log level
    fn color_for_level(&self, level: LogLevel) -> &'static str {
        if !self.use_colors {
            return "";
        }
        
        match level {
            LogLevel::Error => "\x1b[31m", // Red
            LogLevel::Warning => "\x1b[33m", // Yellow
            LogLevel::Info => "\x1b[32m", // Green
            LogLevel::Debug => "\x1b[36m", // Cyan
            LogLevel::Trace => "\x1b[35m", // Magenta
        }
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, level: LogLevel, message: &str) {
        if level > self.level {
            return;
        }
        
        let color = self.color_for_level(level);
        let reset = if self.use_colors { "\x1b[0m" } else { "" };
        let level_str = format!("{:?}", level).to_uppercase();
        
        match level {
            LogLevel::Error | LogLevel::Warning => {
                eprintln!("{}[{}]{} {}", color, level_str, reset, message);
            }
            _ => {
                println!("{}[{}]{} {}", color, level_str, reset, message);
            }
        }
    }
}
```

**Responsibilities:**
- Defines a common logging interface with level-based filtering
- Provides convenience methods for different log levels
- Implements a console logger with optional color support
- Directs error/warning messages to stderr and other messages to stdout

**Implementation Notes:**
- The `Logger` trait is thread-safe (Send + Sync)
- Default implementations for level-specific methods simplify custom loggers
- ConsoleLogger includes color-coding based on log level

### Error Handling (`mod.rs`)

Centralized error handling for Core operations:

```rust
/// Core module errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    /// Environment error
    #[error("Environment error: {0}")]
    Environment(String),
    
    /// FFmpeg executable not found
    #[error("FFmpeg executable not found")]
    FFmpegNotFound,
    
    /// FFprobe executable not found
    #[error("FFprobe executable not found")]
    FFprobeNotFound,
}

/// Result type for Core operations
pub type Result<T> = std::result::Result<T, Error>;
```

**Responsibilities:**
- Defines a central error type for Core operations
- Provides automatic conversion from std::io::Error
- Creates a project-specific Result type alias

**Implementation Notes:**
- Uses `thiserror` for deriving Error implementation
- Includes specific variants for configuration and environment issues
- Provides descriptive error messages for all variants

## Dependencies

The Core module has minimal external dependencies:

- **Standard Library**: Uses `std::io`, `std::path`, `std::sync::Arc`
- **Thiserror**: For error enum implementation

Internal dependencies:
- `ConsoleLogger` depends on the `Logger` trait and `LogLevel` enum.
- `Context` depends on `Config` and `Logger`.
- All components may use `Error` and `Result`.

## Implementation Details

### Ownership Model

- `Config` is designed to be cloneable, enabling shared ownership.
- `Logger` implementations are wrapped in `Arc` for thread-safe sharing.
- `Context` is cloneable but maintains immutable references to its components.

### Thread Safety

- `Logger` trait requires Send + Sync to ensure thread safety.
- `Context` can be safely shared between threads.
- No mutable static or global state is maintained.

### Error Handling Strategy

- All operations that can fail return `Result<T, Error>`.
- Errors include contextual information where appropriate.
- Low-level errors (like IO errors) are wrapped in the application-specific error type.

## Usage Examples

### Creating a Default Context

```rust
let context = Context::default();
context.logger.info("Application started");
```

### Custom Configuration

```rust
let mut config = Config::new();
config.verbosity = 3; // Debug level
config.use_colors = false;

let logger = Arc::new(ConsoleLogger::new(config.log_level(), config.use_colors));
let context = Context::new(config, logger);
```

### Logging Messages

```rust
// These methods check the current log level
context.logger.error("Critical failure");
context.logger.warning("Potential issue detected");
context.logger.info("Processing completed");
context.logger.debug("Variable value: {}", value);
context.logger.trace("Entering function");
```

## Future Enhancements

1. **Configuration Persistence**
   - Add serialization/deserialization for the Config struct
   - Support loading config from files and environment variables
   - Implement config validation

2. **Advanced Logging**
   - Add file-based logger implementation
   - Support log rotation and archiving
   - Add structured logging capabilities

3. **Performance Metrics**
   - Integrate optional performance tracking
   - Provide execution timing and resource usage metrics

4. **Internationalization**
   - Add support for message translation
   - Implement locale-specific formatting

This implementation provides a solid foundation for the Core module while maintaining simplicity and focusing on the most essential functionalities needed by other modules. 