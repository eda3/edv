/// Core functionality for the edv application.
///
/// This module provides the core abstractions and utilities used throughout
/// the application, including error handling, logging, and configuration.
// Modules
pub mod console;

use std::fs;
use std::io;
use std::path::PathBuf;

// Re-exports
pub use self::console::ConsoleLogger;

/// Error type for core operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Missing argument
    #[error("Missing argument: {0}")]
    MissingArgument(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Result type for core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Log levels for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Error level - critical errors
    Error,
    /// Warning level - recoverable issues
    Warning,
    /// Info level - general information
    Info,
    /// Debug level - detailed debugging information
    Debug,
}

/// Trait for logger implementations
pub trait Logger: Send + Sync + std::fmt::Debug {
    /// Logs a message at the specified level
    fn log(&self, level: LogLevel, message: &str);

    /// Logs an error message
    fn error(&self, message: &str);

    /// Logs a warning message
    fn warning(&self, message: &str);

    /// Logs an info message
    fn info(&self, message: &str);

    /// Logs a debug message
    fn debug(&self, message: &str);

    /// Clone the logger
    fn clone_box(&self) -> Box<dyn Logger>;
}

impl Clone for Box<dyn Logger> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Log level
    pub log_level: LogLevel,
    /// Working directory
    pub working_dir: PathBuf,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Default output directory
    pub output_dir: PathBuf,
}

impl Config {
    /// Creates a new configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::Info,
            working_dir: PathBuf::from("."),
            cache_dir: PathBuf::from(".cache"),
            output_dir: PathBuf::from("output"),
        }
    }

    /// Loads configuration from the specified file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// Loaded configuration or error
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        // This is a placeholder implementation
        // In a real application, this would read and parse a config file
        let mut config = Config::new();

        // Check if the file exists
        if !path.exists() {
            return Err(Error::InvalidConfiguration(format!(
                "Config file not found: {:?}",
                path
            )));
        }

        // Set working directory to the parent of the config file
        if let Some(parent) = path.parent() {
            config.working_dir = parent.to_path_buf();
        }

        Ok(config)
    }

    /// Loads default configuration
    ///
    /// # Returns
    ///
    /// Default configuration or error
    ///
    /// # Errors
    ///
    /// Returns an error if default configuration cannot be determined
    pub fn load_default() -> Result<Self> {
        // This is a placeholder implementation
        // In a real application, this would determine default paths
        let config = Config::new();

        // Ensure cache directory exists
        fs::create_dir_all(&config.cache_dir).map_err(Error::Io)?;

        // Ensure output directory exists
        fs::create_dir_all(&config.output_dir).map_err(Error::Io)?;

        Ok(config)
    }

    /// Sets the log level
    ///
    /// # Arguments
    ///
    /// * `level` - New log level
    ///
    /// # Returns
    ///
    /// Updated configuration
    #[must_use]
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Sets the working directory
    ///
    /// # Arguments
    ///
    /// * `dir` - New working directory
    ///
    /// # Returns
    ///
    /// Updated configuration
    #[must_use]
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = dir;
        self
    }

    /// Sets the cache directory
    ///
    /// # Arguments
    ///
    /// * `dir` - New cache directory
    ///
    /// # Returns
    ///
    /// Updated configuration
    #[must_use]
    pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = dir;
        self
    }

    /// Sets the output directory
    ///
    /// # Arguments
    ///
    /// * `dir` - New output directory
    ///
    /// # Returns
    ///
    /// Updated configuration
    #[must_use]
    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = dir;
        self
    }
}

/// Application context
///
/// This structure contains configuration and runtime services needed by commands
#[derive(Debug, Clone)]
pub struct Context {
    /// Application configuration
    pub config: Config,
    /// Logger for application messages
    pub logger: Box<dyn Logger>,
}

impl Context {
    /// Creates a new application context
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `logger` - Logger for application messages
    ///
    /// # Returns
    ///
    /// New application context
    #[must_use]
    pub fn new(config: Config, logger: Box<dyn Logger>) -> Self {
        Self { config, logger }
    }

    /// Logs an error message
    ///
    /// # Arguments
    ///
    /// * `message` - Message to log
    pub fn error(&self, message: &str) {
        self.logger.error(message);
    }

    /// Logs a warning message
    ///
    /// # Arguments
    ///
    /// * `message` - Message to log
    pub fn warning(&self, message: &str) {
        self.logger.warning(message);
    }

    /// Logs an info message
    ///
    /// # Arguments
    ///
    /// * `message` - Message to log
    pub fn info(&self, message: &str) {
        self.logger.info(message);
    }

    /// Logs a debug message
    ///
    /// # Arguments
    ///
    /// * `message` - Message to log
    pub fn debug(&self, message: &str) {
        self.logger.debug(message);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
