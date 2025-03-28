/// Console logger implementation.
///
/// This module provides a console-based logger implementation that outputs
/// log messages to the terminal. It supports different log levels and
/// can be configured to show or hide certain types of messages.
use super::{LogLevel, Logger};

/// Console logger for terminal output.
#[derive(Debug, Clone)]
pub struct ConsoleLogger {
    /// Current log level
    level: LogLevel,
}

impl ConsoleLogger {
    /// Creates a new console logger with the specified log level.
    ///
    /// # Arguments
    ///
    /// * `level` - The log level to use for filtering messages
    ///
    /// # Returns
    ///
    /// A new `ConsoleLogger` instance
    #[must_use]
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, level: LogLevel, message: &str) {
        // Skip messages below the current log level
        if level < self.level {
            return;
        }

        let prefix = match level {
            LogLevel::Error => "[ERROR]",
            LogLevel::Warning => "[WARN]",
            LogLevel::Info => "[INFO]",
            LogLevel::Debug => "[DEBUG]",
        };

        eprintln!("{} {}", prefix, message);
    }

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

    fn clone_box(&self) -> Box<dyn Logger> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_filtering() {
        // Create a logger with Info level
        let logger = ConsoleLogger::new(LogLevel::Info);

        // Debug messages should be filtered out
        logger.debug("This should not be logged");

        // Info messages should be logged
        logger.info("This should be logged");

        // Error messages should be logged
        logger.error("This should be logged");
    }
}
