/// Main application entry point and command dispatcher.
///
/// This module contains the core application structure and entry point
/// for running the CLI tool. It handles initialization, command dispatch,
/// and coordination between various components of the application.
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::core::console::ConsoleLogger;
use crate::core::{Config, Context, LogLevel, Logger};

use super::{CommandRegistry, Result};
use crate::cli::commands;

/// CLI application structure.
///
/// This structure represents the main application and contains
/// the command registry, configuration, and other core components.
pub struct App {
    /// Command registry containing all available commands
    command_registry: CommandRegistry,
    /// Application configuration
    config: Config,
    /// Logger for application messages
    logger: Box<dyn Logger>,
}

/// Command line arguments parser using clap.
///
/// This structure defines the command line interface for the application
/// and is used to parse user input into structured commands and options.
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

/// Subcommands supported by the application.
///
/// This enum defines all the available commands that can be executed
/// by the application, along with their specific arguments.
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
}

impl App {
    /// Creates a new application instance with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `logger` - Logger for application messages
    ///
    /// # Returns
    ///
    /// A new `App` instance.
    pub fn new(config: Config, logger: Box<dyn Logger>) -> Self {
        Self {
            command_registry: CommandRegistry::new(),
            config,
            logger,
        }
    }

    /// Initializes the application, registering all available commands.
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or failure.
    pub fn initialize(&mut self) -> Result<()> {
        // Register all commands
        self.register_commands()?;

        self.logger.info("Application initialized");
        Ok(())
    }

    /// Registers all available commands with the command registry.
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or failure.
    fn register_commands(&mut self) -> Result<()> {
        // TODO: Uncomment these when other commands are implemented
        // self.command_registry.register(Box::new(commands::TrimCommand::new()))?;
        // self.command_registry.register(Box::new(commands::ConcatCommand::new()))?;
        // self.command_registry.register(Box::new(commands::ConvertCommand::new()))?;

        // Register info command
        self.command_registry
            .register(Box::new(commands::InfoCommand::new()))?;

        // Register our render command
        self.command_registry
            .register(Box::new(commands::RenderCommand::new()))?;

        Ok(())
    }

    /// Executes the given command with its arguments.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Returns
    ///
    /// `Result<()>` indicating success or failure.
    pub fn execute_command(&self, command: Commands) -> Result<()> {
        // Create execution context (always create it once here)
        let context = self.create_execution_context()?;

        // Match on command type and execute appropriate handler
        match command {
            Commands::Trim {
                input,
                output,
                start,
                end,
                recompress,
            } => {
                self.logger.debug(&format!(
                    "Executing trim command: input={}, output={}, start={:?}, end={:?}, recompress={}",
                    input, output, start, end, recompress
                ));

                // Command implementation will be added later
                // This is a placeholder to avoid unused variable warnings
                self.logger.info("Trim command executed successfully");
            }
            Commands::Concat {
                input,
                output,
                recompress,
            } => {
                self.logger.debug(&format!(
                    "Executing concat command: input={:?}, output={}, recompress={}",
                    input, output, recompress
                ));

                // Command implementation will be added later
                // This is a placeholder to avoid unused variable warnings
                self.logger.info("Concat command executed successfully");
            }
            Commands::Info { input, detailed } => {
                self.logger.debug(&format!(
                    "Executing info command: input={}, detailed={}",
                    input, detailed
                ));

                // Get the InfoCommand from the registry and execute it
                if let Ok(info_cmd) = self.command_registry.get("info") {
                    // Build the arguments list
                    let mut args = vec![input];
                    if detailed {
                        args.push("--detailed".to_string());
                    }

                    // Execute the command with arguments and the already created context
                    info_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("info".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Creates an execution context for command execution.
    ///
    /// # Returns
    ///
    /// `Result<Context>` containing the execution context or an error.
    fn create_execution_context(&self) -> Result<Context> {
        // Context creation will be implemented when the Context type is available
        // This is a placeholder to avoid compiler warnings
        Ok(Context::new(self.config.clone(), self.logger.clone()))
    }
}

/// Runs the application with command line arguments.
///
/// This function is the main entry point for the CLI application.
/// It parses command line arguments, initializes the application,
/// and executes the requested command.
///
/// # Returns
///
/// `Result<()>` indicating success or failure.
pub fn run() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Determine log level based on verbose flag
    let log_level = if cli.verbose {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    // Create logger
    let logger = Box::new(ConsoleLogger::new(log_level));

    // Load configuration
    let config = match cli.config {
        Some(ref path) => Config::load_from_file(path)?,
        None => Config::load_default()?,
    };

    // Create and initialize the application
    let mut app = App::new(config, logger);
    app.initialize()?;

    // Execute the requested command
    app.execute_command(cli.command)
}
