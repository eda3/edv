/// CLI module for the edv video editing tool.
///
/// This module provides the command-line interface for the application,
/// handling command parsing, execution, and user interaction. It implements
/// a consistent and intuitive CLI experience for video editing operations.
///
/// The module is organized into submodules:
/// - `app`: Main application entry point and command dispatcher
/// - `commands`: Implementation of individual commands
/// - `args`: Command argument parsing utilities
/// - `output`: Terminal output handling

mod app;
mod commands;
mod args;
mod output;
mod utils;

// Public exports
pub use app::{App, run};
pub use commands::{Command, CommandRegistry};
pub use output::{ProgressReporter, OutputFormatter};

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
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] crate::core::Error),
}

/// Result type for CLI operations
pub type Result<T> = std::result::Result<T, Error>; 