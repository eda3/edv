/// `FFmpeg` command construction utilities.
///
/// This module provides a simplified interface for building `FFmpeg` commands.
use crate::ffmpeg::{Error, FFmpeg, Result};
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
        // 一般的なコマンドでは、ある程度のオプションが想定されるため、
        // 初期容量を設定してメモリの再割り当てを減らす
        const INITIAL_CAPACITY: usize = 8;

        Self {
            ffmpeg,
            input_options: Vec::with_capacity(INITIAL_CAPACITY),
            inputs: Vec::with_capacity(2), // 一般的には1-2の入力ファイル
            filter_complex: None,
            output_options: Vec::with_capacity(INITIAL_CAPACITY),
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
        // 必要に応じて容量を確保
        let options_iter = options.into_iter();
        let (lower, upper) = options_iter.size_hint();
        let estimated_size = upper.unwrap_or(lower);

        if estimated_size > 0 {
            self.input_options.reserve(estimated_size);
        }

        self.input_options
            .extend(options_iter.map(|s| s.as_ref().to_string()));
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
    /// Returns an error if the command fails to execute or returns a non-zero exit code.
    pub fn execute(&self) -> Result<()> {
        // バリデーションチェック
        if self.output.is_none() {
            return Err(Error::MissingArgument(
                "No output file specified".to_string(),
            ));
        }

        if self.inputs.is_empty() {
            return Err(Error::MissingArgument(
                "No input files specified".to_string(),
            ));
        }

        // コマンドの構築
        let mut command = Command::new(self.ffmpeg.path());

        // まず、ストリーム処理を効率化するためにバッファリングを設定
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // 入力オプションとファイル
        if !self.input_options.is_empty() {
            command.args(&self.input_options);
        }

        for input in &self.inputs {
            command.arg("-i").arg(input);
        }

        // フィルター複合体がある場合
        if let Some(filter) = &self.filter_complex {
            command.arg("-filter_complex").arg(filter);
        }

        // 出力オプション
        if !self.output_options.is_empty() {
            command.args(&self.output_options);
        }

        // 上書きフラグがある場合は-yフラグを追加
        if self.overwrite {
            command.arg("-y");
        }

        // 最後に出力ファイルを追加
        // (unwrapは問題ありません。既に上で出力が存在するか確認しているため)
        command.arg(self.output.as_ref().unwrap());

        // コマンドを実行
        let output = command.output().map_err(|e| Error::IoError(e))?;

        // 終了コードをチェック
        if !output.status.success() {
            // FFmpegのエラーメッセージを取得して返す
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_message = stderr
                .lines()
                .filter(|line| line.contains("Error") || line.contains("Invalid"))
                .collect::<Vec<_>>()
                .join("\n");

            if !error_message.is_empty() {
                return Err(Error::ExecutionError(error_message));
            }

            return Err(Error::ProcessTerminated {
                exit_code: output.status.code(),
                message: format!("FFmpeg process failed with exit code: {}", output.status),
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
