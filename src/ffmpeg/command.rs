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
    pub fn output<P: AsRef<Path>>(&mut self, output: P) -> &mut Self {
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

    /// Executes the `FFmpeg` command.
    ///
    /// Returns an error if:
    /// * No output file is specified
    /// * No input files are specified
    /// * The `FFmpeg` process fails to start or returns a non-zero exit code
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
}
