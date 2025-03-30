use crate::ffmpeg::{Error, FFmpeg, Result};
/// `FFmpeg` command construction utilities.
///
/// This module provides a simplified interface for building `FFmpeg` commands.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Represents an `FFmpeg` command.
#[derive(Debug, Clone)]
pub struct FFmpegCommand<'a> {
    /// The `FFmpeg` instance to use.
    ffmpeg: &'a FFmpeg,
    /// Input options to apply before specifying inputs.
    input_options: Vec<String>,
    /// Input files for the command.
    inputs: Vec<PathBuf>,
    /// Filter complex to apply (if any).
    filter_complex: Option<String>,
    /// Output options to apply before specifying output.
    output_options: Vec<String>,
    /// Output file for the command.
    output: Option<PathBuf>,
    /// Whether to overwrite output file if it exists.
    overwrite: bool,
}

impl<'a> FFmpegCommand<'a> {
    /// Creates a new `FFmpeg` command.
    ///
    /// # Arguments
    ///
    /// * `ffmpeg` - The `FFmpeg` instance to use
    #[must_use]
    pub fn new(ffmpeg: &'a FFmpeg) -> Self {
        Self {
            ffmpeg,
            input_options: Vec::new(),
            inputs: Vec::new(),
            filter_complex: None,
            output_options: Vec::new(),
            output: None,
            overwrite: false,
        }
    }

    /// Adds input options to be applied before an input file.
    ///
    /// # Arguments
    ///
    /// * `options` - The options to add
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn input_options<S: AsRef<str>, I: IntoIterator<Item = S>>(
        &mut self,
        options: I,
    ) -> &mut Self {
        self.input_options
            .extend(options.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Adds an input file to the command.
    ///
    /// # Arguments
    ///
    /// * `input` - The input file path
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn input<P: AsRef<Path>>(&mut self, input: P) -> &mut Self {
        self.inputs.push(input.as_ref().to_path_buf());
        self
    }

    /// Sets a filter complex for the command.
    ///
    /// # Arguments
    ///
    /// * `filter` - The filter complex string
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn filter_complex<S: AsRef<str>>(&mut self, filter: S) -> &mut Self {
        self.filter_complex = Some(filter.as_ref().to_string());
        self
    }

    /// Adds output options to be applied before the output file.
    ///
    /// # Arguments
    ///
    /// * `options` - The options to add
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn output_options<S: AsRef<str>, I: IntoIterator<Item = S>>(
        &mut self,
        options: I,
    ) -> &mut Self {
        self.output_options
            .extend(options.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Sets the output file for the command.
    ///
    /// # Arguments
    ///
    /// * `output` - The output file path
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn set_output<P: AsRef<Path>>(&mut self, output: P) -> &mut Self {
        self.output = Some(output.as_ref().to_path_buf());
        self
    }

    /// Sets whether to overwrite the output file if it exists.
    ///
    /// # Arguments
    ///
    /// * `overwrite` - Whether to overwrite
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }

    /// Adds an input option to be applied before an input file.
    ///
    /// # Arguments
    ///
    /// * `option` - The option name (e.g., "-ss")
    /// * `value` - The option value (e.g., "10.5")
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn add_input_option<S: AsRef<str>, T: AsRef<str>>(
        &mut self,
        option: S,
        value: T,
    ) -> &mut Self {
        self.input_options.push(option.as_ref().to_string());
        self.input_options.push(value.as_ref().to_string());
        self
    }

    /// Adds an output option to be applied before the output file.
    ///
    /// # Arguments
    ///
    /// * `option` - The option name (e.g., "-c")
    /// * `value` - The option value (e.g., "copy")
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn add_output_option<S: AsRef<str>, T: AsRef<str>>(
        &mut self,
        option: S,
        value: T,
    ) -> &mut Self {
        self.output_options.push(option.as_ref().to_string());
        self.output_options.push(value.as_ref().to_string());
        self
    }

    /// Adds an input file to the command.
    ///
    /// # Arguments
    ///
    /// * `input` - The input file path
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn add_input<P: AsRef<Path>>(&mut self, input: P) -> &mut Self {
        self.inputs.push(input.as_ref().to_path_buf());
        self
    }

    /// Executes the `FFmpeg` command.
    ///
    /// Returns an error if:
    /// * No output file is specified
    /// * No input files are specified
    /// * The `FFmpeg` process fails to start or returns a non-zero exit code
    ///
    /// # Errors
    ///
    /// This function can return the following errors:
    /// * `Error::MissingArgument` - If no input files are specified or no output file is specified
    /// * `Error::IoError` - If the `FFmpeg` process fails to start
    /// * `Error::ProcessTerminated` - If the `FFmpeg` process returns a non-zero exit code
    ///
    /// # Panics
    ///
    /// This function should not panic as it performs explicit checks before using `unwrap()`.
    /// If `self.output` is `None`, it returns an error before attempting to unwrap.
    pub fn execute(&self) -> Result<()> {
        // Check that we have inputs and an output
        if self.inputs.is_empty() {
            return Err(Error::MissingArgument(
                "No input files specified".to_string(),
            ));
        }

        if self.output.is_none() {
            return Err(Error::MissingArgument(
                "No output file specified".to_string(),
            ));
        }

        // Build the command
        let mut cmd = Command::new(self.ffmpeg.path());

        // Add global options
        if self.overwrite {
            cmd.arg("-y");
        }

        // Add input options and inputs
        for i in 0..self.inputs.len() {
            // Add input options for this input
            if i == 0 && !self.input_options.is_empty() {
                for option in &self.input_options {
                    cmd.arg(option);
                }
            }

            // Add the input
            cmd.arg("-i").arg(&self.inputs[i]);
        }

        // Add filter complex if specified
        if let Some(filter) = &self.filter_complex {
            cmd.arg("-filter_complex").arg(filter);
        }

        // Add output options
        for option in &self.output_options {
            cmd.arg(option);
        }

        // Add output
        cmd.arg(self.output.as_ref().unwrap());

        // Execute the command
        let output = cmd
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .map_err(Error::IoError)?;

        // Check for success
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::ProcessTerminated {
                exit_code: output.status.code(),
                message: format!("FFmpeg process failed: {stderr}"),
            });
        }

        Ok(())
    }

    /// Executes the FFmpeg command with progress callback.
    ///
    /// # Arguments
    ///
    /// * `progress_callback` - Callback function that receives FFmpeg progress output
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    ///
    /// # Errors
    ///
    /// Returns an error if the FFmpeg process fails
    pub fn execute_with_progress<F>(&self, mut progress_callback: F) -> Result<()>
    where
        F: FnMut(&str),
    {
        // Check that we have inputs and an output
        if self.inputs.is_empty() {
            return Err(Error::MissingArgument(
                "No input files specified".to_string(),
            ));
        }

        if self.output.is_none() {
            return Err(Error::MissingArgument(
                "No output file specified".to_string(),
            ));
        }

        // Build the command
        let mut cmd = Command::new(self.ffmpeg.path());

        // Add global options
        if self.overwrite {
            cmd.arg("-y");
        }

        // Add progress option for stderr output
        cmd.arg("-progress").arg("pipe:2");

        // Add input options and inputs
        for i in 0..self.inputs.len() {
            // Add input options for this input
            if i == 0 && !self.input_options.is_empty() {
                for option in &self.input_options {
                    cmd.arg(option);
                }
            }

            // Add the input
            cmd.arg("-i").arg(&self.inputs[i]);
        }

        // Add filter complex if specified
        if let Some(filter) = &self.filter_complex {
            cmd.arg("-filter_complex").arg(filter);
        }

        // Add output options
        for option in &self.output_options {
            cmd.arg(option);
        }

        // Add output
        cmd.arg(self.output.as_ref().unwrap());

        // Execute the command
        let mut child = cmd
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(Error::IoError)?;

        // Read progress updates from stderr
        if let Some(stderr) = child.stderr.take() {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);

            // Process each line as it becomes available
            for line in reader.lines() {
                if let Ok(line) = line {
                    progress_callback(&line);
                }
            }
        }

        // Wait for the process to finish
        let output = child.wait_with_output().map_err(Error::IoError)?;

        // Check for success
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::ProcessTerminated {
                exit_code: output.status.code(),
                message: format!("FFmpeg process failed: {stderr}"),
            });
        }

        Ok(())
    }
}
