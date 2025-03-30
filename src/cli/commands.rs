/// Command definitions and registry for the CLI application.
///
/// This module defines the command trait that all edv commands must implement,
/// as well as the command registry that manages available commands. It serves
/// as the core abstraction for command execution in the application.
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::Path;

use crate::core::Context;

use super::{Error, Result};

/// Trait that all commands must implement.
///
/// This trait defines the interface for all commands in the application.
/// Each command must provide its name, description, and an execution method
/// that performs the command's actual functionality.
pub trait Command: Send + Sync + Debug {
    /// Gets the name of the command.
    ///
    /// This name is used for command registration and lookup.
    fn name(&self) -> &str;

    /// Gets a human-readable description of the command.
    ///
    /// This description is used for help text and documentation.
    fn description(&self) -> &str;

    /// Gets usage examples for the command.
    ///
    /// These examples are displayed in help text to guide users.
    fn usage(&self) -> &str;

    /// Executes the command with the given context and arguments.
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context containing configuration and services
    /// * `args` - Command-specific arguments parsed from the command line
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or failure of the command.
    fn execute(&self, context: &Context, args: &[String]) -> Result<()>;
}

/// Registry for managing available commands.
///
/// The command registry maintains a collection of all available commands
/// and provides methods for registering, looking up, and listing commands.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    /// Map of command names to command implementations
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    /// Creates a new, empty command registry.
    ///
    /// # Returns
    ///
    /// A new `CommandRegistry` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Registers a command with the registry.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to register
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or whether the command name was already registered.
    ///
    /// # Errors
    ///
    /// Returns an error if a command with the same name is already registered.
    pub fn register(&mut self, command: Box<dyn Command>) -> Result<()> {
        let name = command.name().to_string();

        if self.commands.contains_key(&name) {
            return Err(Error::DuplicateCommand(name));
        }

        self.commands.insert(name, command);
        Ok(())
    }

    /// Gets a command by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the command to retrieve
    ///
    /// # Returns
    ///
    /// `Result<&dyn Command>` containing a reference to the command if found.
    ///
    /// # Errors
    ///
    /// Returns an error if no command with the given name is registered.
    pub fn get(&self, name: &str) -> Result<&dyn Command> {
        self.commands
            .get(name)
            .map(AsRef::as_ref)
            .ok_or_else(|| Error::UnknownCommand(name.to_string()))
    }

    /// Gets a list of all registered commands.
    ///
    /// # Returns
    ///
    /// A vector of references to all registered commands.
    #[must_use]
    pub fn list(&self) -> Vec<&dyn Command> {
        self.commands.values().map(AsRef::as_ref).collect()
    }

    /// Gets a list of all command names.
    ///
    /// # Returns
    ///
    /// A vector of command names.
    #[must_use]
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Checks if a command with the given name is registered.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the command to check
    ///
    /// # Returns
    ///
    /// `true` if a command with the given name is registered, `false` otherwise.
    #[must_use]
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Gets the number of registered commands.
    ///
    /// # Returns
    ///
    /// The number of commands in the registry.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

/// Renders a project to a video file.
#[derive(Debug)]
pub struct RenderCommand;

impl RenderCommand {
    /// Creates a new render command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for RenderCommand {
    fn name(&self) -> &str {
        "render"
    }

    fn description(&self) -> &str {
        "Renders a project to a video file"
    }

    fn usage(&self) -> &str {
        "render --project <project_file> --output <output_file> [options]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // For now, return a simple not implemented error
        // The actual implementation will be added in a separate PR
        context.logger.info("Render command received");
        context.logger.info(&format!("Args: {:?}", args));

        // Return success for now - this is just a stub until fully implemented
        Ok(())
    }
}

/// Display information about a media file.
#[derive(Debug)]
pub struct InfoCommand;

impl InfoCommand {
    /// Creates a new info command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Command for InfoCommand {
    fn name(&self) -> &str {
        "info"
    }

    fn description(&self) -> &str {
        "Display information about a media file"
    }

    fn usage(&self) -> &str {
        "info <file_path> [--detailed]"
    }

    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::MissingArgument("file path".to_string()));
        }

        let file_path = &args[0];
        let path = Path::new(file_path);

        // Check if file exists
        if !path.exists() {
            return Err(Error::InvalidPath(format!("File not found: {}", file_path)));
        }

        // Get basic file information
        context
            .logger
            .info(&format!("File information for: {}", file_path));

        if let Ok(metadata) = fs::metadata(path) {
            let size_bytes = metadata.len();
            let size_kb = size_bytes as f64 / 1024.0;
            let size_mb = size_kb / 1024.0;

            context.logger.info(&format!("File exists: Yes"));
            context.logger.info(&format!(
                "File size: {} bytes ({:.2} KB, {:.2} MB)",
                size_bytes, size_kb, size_mb
            ));

            // Get MIME type (if can be guessed)
            if let Some(file_type) = mime_guess::from_path(path).first_raw() {
                context
                    .logger
                    .info(&format!("MIME type (guessed): {}", file_type));
            }

            // Show if it's a directory
            if metadata.is_dir() {
                context.logger.info("Type: Directory");
            } else if metadata.is_file() {
                context.logger.info("Type: Regular file");
            }

            // Show if detailed mode is requested
            let detailed = args.len() > 1 && (args[1] == "--detailed" || args[1] == "-d");
            if detailed {
                if let Ok(last_modified) = metadata.modified() {
                    let last_modified_str = chrono::DateTime::<chrono::Local>::from(last_modified)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();
                    context
                        .logger
                        .info(&format!("Last modified: {}", last_modified_str));
                }

                // In a real implementation, we would use FFmpeg here to get media-specific details
                context
                    .logger
                    .info("Note: Full media information requires FFmpeg integration (coming soon)");
            }
        } else {
            return Err(Error::InvalidPath(format!(
                "Could not read file metadata: {}",
                file_path
            )));
        }

        context.logger.info("Info command executed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock command for testing
    #[derive(Debug)]
    struct MockCommand {
        name: String,
        description: String,
        usage: String,
    }

    impl Command for MockCommand {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn usage(&self) -> &str {
            &self.usage
        }

        fn execute(&self, _context: &Context, _args: &[String]) -> Result<()> {
            // Mock implementation that does nothing
            Ok(())
        }
    }

    impl MockCommand {
        fn new(name: &str, description: &str, usage: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
                usage: usage.to_string(),
            }
        }
    }

    #[test]
    fn test_register_and_get_command() {
        let mut registry = CommandRegistry::new();
        let command = MockCommand::new("test", "Test command", "test --arg value");
        let command_name = command.name().to_string();

        // Register the command
        registry.register(Box::new(command)).unwrap();

        // Verify command is in registry
        assert!(registry.has_command(&command_name));
        assert_eq!(registry.command_count(), 1);

        // Get the command
        let cmd = registry.get(&command_name).unwrap();
        assert_eq!(cmd.name(), "test");
        assert_eq!(cmd.description(), "Test command");
        assert_eq!(cmd.usage(), "test --arg value");
    }

    #[test]
    fn test_duplicate_command_registration() {
        let mut registry = CommandRegistry::new();
        let command1 = MockCommand::new("test", "First test command", "test1");
        let command2 = MockCommand::new("test", "Second test command", "test2");

        // Register the first command
        registry.register(Box::new(command1)).unwrap();

        // Try to register a command with the same name
        let result = registry.register(Box::new(command2));
        assert!(result.is_err());

        // Ensure only one command is registered
        assert_eq!(registry.command_count(), 1);
    }

    #[test]
    fn test_unknown_command() {
        let registry = CommandRegistry::new();

        // Try to get a non-existent command
        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_commands() {
        let mut registry = CommandRegistry::new();
        let command1 = MockCommand::new("test1", "First test command", "test1");
        let command2 = MockCommand::new("test2", "Second test command", "test2");

        // Register commands
        registry.register(Box::new(command1)).unwrap();
        registry.register(Box::new(command2)).unwrap();

        // List commands
        let commands = registry.list();
        assert_eq!(commands.len(), 2);

        // Get command names
        let names = registry.command_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"test1".to_string()));
        assert!(names.contains(&"test2".to_string()));
    }
}
