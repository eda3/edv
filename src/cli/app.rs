/// Main application entry point and command dispatcher.
///
/// This module contains the core application structure and entry point
/// for running the CLI tool. It handles initialization, command dispatch,
/// and coordination between various components of the application.
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use log::debug;

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
    /// Trims a video file to the specified start and end times
    Trim {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Start time in seconds
        #[arg(short, long)]
        start: f64,

        /// End time in seconds
        #[arg(short, long)]
        end: f64,
    },

    /// Displays information about a video file
    Info {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Renders a project to a video file
    Render {
        /// Project file path
        #[arg(short, long)]
        project: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Undoes the last edit in a project
    ProjectUndo {
        /// Project file path
        #[arg(short, long)]
        project: PathBuf,
    },

    /// Redoes the last undone edit in a project
    ProjectRedo {
        /// Project file path
        #[arg(short, long)]
        project: PathBuf,
    },

    /// Plays a video file with optional start and end times
    Play {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Start time in format HH:MM:SS or seconds
        #[arg(short, long)]
        start: Option<String>,

        /// End time in format HH:MM:SS or seconds
        #[arg(short, long)]
        end: Option<String>,
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
        debug!("Initializing application");

        // Register commands
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
        // Register trim command
        self.command_registry
            .register(Box::new(commands::TrimCommand::new()))?;

        // TODO: Uncomment these when other commands are implemented
        // self.command_registry.register(Box::new(commands::ConcatCommand::new()))?;
        // self.command_registry.register(Box::new(commands::ConvertCommand::new()))?;

        // Register info command
        self.command_registry
            .register(Box::new(commands::InfoCommand::new()))?;

        // Register our render command
        self.command_registry
            .register(Box::new(commands::RenderCommand::new()))?;

        // Register project undo command
        self.command_registry
            .register(Box::new(commands::ProjectUndoCommand::new()))?;

        // Register project redo command
        self.command_registry
            .register(Box::new(commands::ProjectRedoCommand::new()))?;

        // Register play command
        self.command_registry
            .register(Box::new(commands::PlayCommand::new()))?;

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
            } => {
                self.logger.debug(&format!(
                    "Executing trim command: input={}, output={}, start={}, end={}",
                    input.display(),
                    output.display(),
                    start,
                    end
                ));

                // Get the TrimCommand from the registry and execute it
                if let Ok(trim_cmd) = self.command_registry.get("trim") {
                    // Build the arguments list
                    let mut args = vec![
                        input.to_string_lossy().to_string(),
                        output.to_string_lossy().to_string(),
                    ];
                    args.push(start.to_string());
                    args.push(end.to_string());

                    // Execute the command with arguments and the already created context
                    trim_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("trim".to_string()));
                }
            }
            Commands::Info { input } => {
                self.logger.debug(&format!(
                    "Executing info command: input={}",
                    input.display()
                ));

                // Get the InfoCommand from the registry and execute it
                if let Ok(info_cmd) = self.command_registry.get("info") {
                    // Build the arguments list
                    let mut args = vec![input.to_string_lossy().to_string()];

                    // Execute the command with arguments and the already created context
                    info_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("info".to_string()));
                }
            }
            Commands::Render { project, output } => {
                self.logger.debug(&format!(
                    "Executing render command: project={}, output={}",
                    project.display(),
                    output.display()
                ));

                // Get the RenderCommand from the registry and execute it
                if let Ok(render_cmd) = self.command_registry.get("render") {
                    // Build the arguments list
                    let mut args = vec![
                        project.to_string_lossy().to_string(),
                        output.to_string_lossy().to_string(),
                    ];

                    // Execute the command with arguments and the already created context
                    render_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("render".to_string()));
                }
            }
            Commands::ProjectUndo { project } => {
                self.logger.debug(&format!(
                    "Executing project undo command: project={}",
                    project.display()
                ));

                // Get the ProjectUndoCommand from the registry and execute it
                if let Ok(project_undo_cmd) = self.command_registry.get("project-undo") {
                    // Build the arguments list
                    let mut args = vec![project.to_string_lossy().to_string()];

                    // Execute the command with arguments and the already created context
                    project_undo_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("project-undo".to_string()));
                }
            }
            Commands::ProjectRedo { project } => {
                self.logger.debug(&format!(
                    "Executing project redo command: project={}",
                    project.display()
                ));

                // Get the ProjectRedoCommand from the registry and execute it
                if let Ok(project_redo_cmd) = self.command_registry.get("project-redo") {
                    // Build the arguments list
                    let mut args = vec![project.to_string_lossy().to_string()];

                    // Execute the command with arguments and the already created context
                    project_redo_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("project-redo".to_string()));
                }
            }
            Commands::Play { input, start, end } => {
                self.logger.debug(&format!(
                    "Executing play command: input={}, start={:?}, end={:?}",
                    input.display(),
                    start,
                    end
                ));

                // Get the PlayCommand from the registry and execute it
                if let Ok(play_cmd) = self.command_registry.get("play") {
                    // Build the arguments list
                    let mut args = vec![input.to_string_lossy().to_string()];

                    if let Some(start_time) = start {
                        args.push("--start".to_string());
                        args.push(start_time);
                    }

                    if let Some(end_time) = end {
                        args.push("--end".to_string());
                        args.push(end_time);
                    }

                    // Execute the command with arguments and the already created context
                    play_cmd.execute(&context, &args)?;
                } else {
                    return Err(super::Error::UnknownCommand("play".to_string()));
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
