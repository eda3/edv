/// Output formatting and progress display utilities for the CLI.
///
/// This module provides utilities for consistent terminal output formatting,
/// including colorized text, progress indicators, and result formatting.
/// It helps present information to the user in a clear and visually appealing way.

use std::fmt::Display;
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Represents a terminal output level for indicating the importance of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputLevel {
    /// Debug-level information (verbose output).
    Debug,
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
    /// Success message.
    Success,
}

/// Formats a message with optional color and styling based on the output level.
///
/// # Arguments
///
/// * `level` - The output level that determines styling
/// * `message` - The message to format
///
/// # Returns
///
/// A string with appropriate formatting applied.
#[must_use]
pub fn format_message(level: OutputLevel, message: &str) -> String {
    // Simple implementation - in a real app, this would use a terminal color library
    match level {
        OutputLevel::Debug => format!("[DEBUG] {message}"),
        OutputLevel::Info => format!("[INFO] {message}"),
        OutputLevel::Warning => format!("[WARNING] {message}"),
        OutputLevel::Error => format!("[ERROR] {message}"),
        OutputLevel::Success => format!("[SUCCESS] {message}"),
    }
}

/// Prints a message to stdout with appropriate formatting.
///
/// # Arguments
///
/// * `level` - The output level that determines styling
/// * `message` - The message to print
///
/// # Errors
///
/// Returns an IO error if writing to stdout fails.
pub fn print_message(level: OutputLevel, message: &str) -> io::Result<()> {
    let formatted = format_message(level, message);
    let mut stdout = io::stdout();
    writeln!(stdout, "{formatted}")
}

/// Prints an error message to stderr.
///
/// # Arguments
///
/// * `message` - The error message to print
///
/// # Errors
///
/// Returns an IO error if writing to stderr fails.
pub fn print_error(message: &str) -> io::Result<()> {
    let formatted = format_message(OutputLevel::Error, message);
    let mut stderr = io::stderr();
    writeln!(stderr, "{formatted}")
}

/// Prints a warning message to stderr.
///
/// # Arguments
///
/// * `message` - The warning message to print
///
/// # Errors
///
/// Returns an IO error if writing to stderr fails.
pub fn print_warning(message: &str) -> io::Result<()> {
    let formatted = format_message(OutputLevel::Warning, message);
    let mut stderr = io::stderr();
    writeln!(stderr, "{formatted}")
}

/// Prints a success message to stdout.
///
/// # Arguments
///
/// * `message` - The success message to print
///
/// # Errors
///
/// Returns an IO error if writing to stdout fails.
pub fn print_success(message: &str) -> io::Result<()> {
    let formatted = format_message(OutputLevel::Success, message);
    let mut stdout = io::stdout();
    writeln!(stdout, "{formatted}")
}

/// Formats a duration as a human-readable string.
///
/// # Arguments
///
/// * `duration` - The duration to format
///
/// # Returns
///
/// A human-readable string representing the duration.
#[must_use]
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    
    if total_secs < 60 {
        return format!("{:.2}s", duration.as_secs_f64());
    }
    
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else {
        format!("{minutes}m {seconds}s")
    }
}

/// Prints a table to stdout with headers and rows of data.
///
/// # Arguments
///
/// * `headers` - The column headers
/// * `rows` - The rows of data, each row should have the same number of columns as headers
///
/// # Errors
///
/// Returns an IO error if writing to stdout fails.
pub fn print_table<T: Display>(headers: &[&str], rows: &[Vec<T>]) -> io::Result<()> {
    if headers.is_empty() || rows.is_empty() {
        return Ok(());
    }
    
    // Calculate column widths
    let mut col_widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();
    
    for row in rows {
        for (i, item) in row.iter().enumerate() {
            if i < col_widths.len() {
                let item_width = format!("{item}").len();
                if item_width > col_widths[i] {
                    col_widths[i] = item_width;
                }
            }
        }
    }
    
    let mut stdout = io::stdout();
    
    // Print headers
    for (i, header) in headers.iter().enumerate() {
        if i > 0 {
            write!(stdout, " | ")?;
        }
        write!(stdout, "{:width$}", header, width = col_widths[i])?;
    }
    writeln!(stdout)?;
    
    // Print separator
    let total_width: usize = col_widths.iter().sum::<usize>() + (headers.len().saturating_sub(1) * 3);
    writeln!(stdout, "{}", "-".repeat(total_width))?;
    
    // Print rows
    for row in rows {
        for (i, item) in row.iter().enumerate() {
            if i > 0 {
                write!(stdout, " | ")?;
            }
            if i < col_widths.len() {
                write!(stdout, "{:width$}", item, width = col_widths[i])?;
            }
        }
        writeln!(stdout)?;
    }
    
    Ok(())
}

/// A simple progress bar for displaying operation progress.
#[derive(Debug)]
pub struct ProgressBar {
    /// The total number of steps to complete.
    total: usize,
    /// The current step number.
    current: usize,
    /// Width of the progress bar in characters.
    width: usize,
    /// Time when the progress bar was created.
    start_time: Instant,
    /// Last update time.
    last_update: Instant,
    /// The update interval to avoid excessive redraws.
    update_interval: Duration,
    /// Whether the progress bar has been completed.
    completed: bool,
}

impl ProgressBar {
    /// Creates a new progress bar.
    ///
    /// # Arguments
    ///
    /// * `total` - The total number of steps to complete
    /// * `width` - The width of the progress bar in characters
    ///
    /// # Returns
    ///
    /// A new `ProgressBar` instance.
    #[must_use]
    pub fn new(total: usize, width: usize) -> Self {
        let now = Instant::now();
        Self {
            total,
            current: 0,
            width,
            start_time: now,
            last_update: now,
            update_interval: Duration::from_millis(100),
            completed: false,
        }
    }
    
    /// Updates the progress bar to the specified step.
    ///
    /// # Arguments
    ///
    /// * `step` - The current step number
    ///
    /// # Errors
    ///
    /// Returns an IO error if writing to stdout fails.
    pub fn update(&mut self, step: usize) -> io::Result<()> {
        let now = Instant::now();
        self.current = step.min(self.total);
        
        // Don't update too frequently to avoid flickering
        if self.current == self.total {
            self.completed = true;
        } else if !self.completed && now.duration_since(self.last_update) < self.update_interval {
            return Ok(());
        }
        
        self.last_update = now;
        self.draw()
    }
    
    /// Increments the progress bar by one step.
    ///
    /// # Errors
    ///
    /// Returns an IO error if writing to stdout fails.
    pub fn increment(&mut self) -> io::Result<()> {
        if self.current < self.total {
            self.update(self.current + 1)
        } else {
            Ok(())
        }
    }
    
    /// Completes the progress bar.
    ///
    /// # Errors
    ///
    /// Returns an IO error if writing to stdout fails.
    pub fn complete(&mut self) -> io::Result<()> {
        self.update(self.total)?;
        writeln!(io::stdout())
    }
    
    /// Draws the progress bar to stdout.
    ///
    /// # Errors
    ///
    /// Returns an IO error if writing to stdout fails.
    fn draw(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        
        // Calculate progress percentage
        let percent = if self.total == 0 { 100 } else { self.current * 100 / self.total };
        
        // Calculate the number of filled positions
        let filled = if self.total == 0 {
            self.width
        } else {
            self.width * self.current / self.total
        };
        
        // Calculate elapsed time
        let elapsed = self.start_time.elapsed();
        let elapsed_str = format_duration(elapsed);
        
        // Clear line and draw progress
        write!(stdout, "\r")?;
        write!(stdout, "[{filled_bar:filled_width$}{empty_bar:empty_width$}] {percent}% ({current}/{total}) {elapsed}",
               filled_bar = "=".repeat(filled),
               filled_width = filled,
               empty_bar = " ".repeat(self.width - filled),
               empty_width = self.width - filled,
               percent = percent,
               current = self.current,
               total = self.total,
               elapsed = elapsed_str)?;
        
        stdout.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_message() {
        assert_eq!(format_message(OutputLevel::Debug, "test message"), "[DEBUG] test message");
        assert_eq!(format_message(OutputLevel::Info, "test message"), "[INFO] test message");
        assert_eq!(format_message(OutputLevel::Warning, "test message"), "[WARNING] test message");
        assert_eq!(format_message(OutputLevel::Error, "test message"), "[ERROR] test message");
        assert_eq!(format_message(OutputLevel::Success, "test message"), "[SUCCESS] test message");
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30.00s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1h 2m 5s");
    }
    
    #[test]
    fn test_progress_bar_new() {
        let progress = ProgressBar::new(100, 50);
        assert_eq!(progress.total, 100);
        assert_eq!(progress.width, 50);
        assert_eq!(progress.current, 0);
        assert!(!progress.completed);
    }
    
    #[test]
    fn test_progress_bar_update() {
        let mut progress = ProgressBar::new(100, 50);
        
        // We're not actually testing the output, just that it doesn't error
        assert!(progress.update(50).is_ok());
        assert_eq!(progress.current, 50);
        
        // Check overflow protection
        assert!(progress.update(150).is_ok());
        assert_eq!(progress.current, 100);
        assert!(progress.completed);
    }
    
    #[test]
    fn test_progress_bar_increment() {
        let mut progress = ProgressBar::new(100, 50);
        
        // We're not actually testing the output, just that it doesn't error
        assert!(progress.increment().is_ok());
        assert_eq!(progress.current, 1);
        
        // Set near the end
        progress.current = 99;
        assert!(progress.increment().is_ok());
        assert_eq!(progress.current, 100);
        assert!(progress.completed);
        
        // Already at the end
        assert!(progress.increment().is_ok());
        assert_eq!(progress.current, 100);
    }
} 