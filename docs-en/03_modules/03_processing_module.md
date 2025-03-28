# edv - Processing Module Implementation

This document provides detailed implementation guidelines for the Processing module of the edv application, which handles video processing operations through FFmpeg integration.

## Overview

The Processing module serves as the primary interface for video processing operations. It abstracts the complexity of FFmpeg command generation, handles execution, and provides progress reporting for video operations.

## Structure

```
src/processing/
├── mod.rs                 // Module exports
├── ffmpeg/                // FFmpeg integration
│   ├── mod.rs             // FFmpeg exports
│   ├── command.rs         // FFmpeg command builder
│   ├── executor.rs        // Command execution
│   └── parser.rs          // Output parsing
├── pipeline.rs            // Processing pipeline
├── media_info.rs          // Media information extraction
├── operations/            // Operation implementations
│   ├── mod.rs             // Operation exports
│   ├── trim.rs            // Trim operation
│   ├── concat.rs          // Concatenation operation
│   ├── filter.rs          // Filter application
│   ├── convert.rs         // Format conversion
│   └── custom.rs          // Custom operation support
└── progress.rs            // Progress reporting
```

## Key Components

### FFmpeg Wrapper (ffmpeg/mod.rs)

The FFmpeg wrapper handles interaction with the FFmpeg executable:

```rust
pub struct FFmpegWrapper {
    ffmpeg_path: PathBuf,
    temp_dir: PathBuf,
}

impl FFmpegWrapper {
    /// Create a new FFmpeg wrapper
    pub fn new(ffmpeg_path: PathBuf, temp_dir: PathBuf) -> Self {
        Self {
            ffmpeg_path,
            temp_dir,
        }
    }
    
    /// Run an FFmpeg command
    pub fn run_command(&self, command: FFmpegCommand, progress: Option<Box<dyn ProgressReporter>>) -> Result<()> {
        let mut cmd = Command::new(&self.ffmpeg_path);
        
        // Add global options
        cmd.arg("-y"); // Overwrite output files
        cmd.arg("-hide_banner"); // Hide banner
        
        // Add command-specific arguments
        for arg in command.build() {
            cmd.arg(arg);
        }
        
        // Run command with progress reporting
        self.execute_command(cmd, progress)
    }
    
    /// Get media information for a file
    pub fn get_media_info(&self, path: &Path) -> Result<MediaInfo> {
        let output = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(path)
            .output()?;
        
        // Parse output for media info
        MediaInfoParser::parse(&String::from_utf8_lossy(&output.stderr))
    }
    
    /// Execute command with progress reporting
    fn execute_command(&self, mut command: Command, progress: Option<Box<dyn ProgressReporter>>) -> Result<()> {
        // Configure command for progress output
        command.stderr(Stdio::piped());
        
        // Start the process
        let mut child = command.spawn()?;
        
        // Handle progress reporting
        if let Some(mut progress_reporter) = progress {
            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                
                for line in reader.lines() {
                    if let Ok(line) = line {
                        // Parse progress information
                        if let Some(progress_info) = parse_progress(&line) {
                            progress_reporter.update(progress_info.position, Some(progress_info.message));
                        }
                    }
                }
            }
            
            progress_reporter.finish("Processing completed".to_string());
        }
        
        // Wait for the process to finish
        let status = child.wait()?;
        
        if !status.success() {
            return Err(Error::FFmpeg(format!("FFmpeg process failed with exit code: {}", status)));
        }
        
        Ok(())
    }
}
```

### FFmpeg Command Builder (ffmpeg/command.rs)

The command builder creates FFmpeg command lines:

```rust
#[derive(Debug, Clone)]
pub struct FFmpegCommand {
    /// The FFmpeg instance to use.
    ffmpeg: FFmpeg,
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

impl FFmpegCommand {
    /// Creates a new FFmpeg command.
    ///
    /// # Arguments
    ///
    /// * `ffmpeg` - The FFmpeg instance to use
    #[must_use]
    pub fn new(ffmpeg: FFmpeg) -> Self {
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
    pub fn input_options<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, options: I) -> &mut Self {
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
    pub fn output_options<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, options: I) -> &mut Self {
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
    
    /// Executes the FFmpeg command.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * No output file is specified
    /// * No input files are specified
    /// * The FFmpeg process fails to start or returns a non-zero exit code
    pub fn execute(&self) -> Result<()> {
        // Implementation details...
    }
}
```

#### Best Practices for Command Builder Usage

The `FFmpegCommand` API follows Rust's borrowing patterns to prevent ownership issues:

1. **Method Chaining with Mutable References**
   
   Commands can be built using method chaining without taking ownership:
   ```rust
   let mut cmd = FFmpegCommand::new(ffmpeg);
   cmd.input(input_path)
      .output_options(&output_options)
      .output(output_path)
      .overwrite(true);
   
   cmd.execute()?;
   ```

2. **Ownership of String Arguments**
   
   When providing string options, use owned collections instead of short-lived references:
   ```rust
   // Good approach with owned strings
   let output_options = vec![
       "-c:a".to_string(), 
       "aac".to_string(),
       "-b:a".to_string(),
       "128k".to_string()
   ];
   cmd.output_options(&output_options);
   
   // Bad approach with temporary strings
   // cmd.output_options(&["-c:a", &codec.to_string()]);  // Error: temporary value dropped
   ```

3. **Multiple Commands From Single Template**
   
   When you need to reuse a command template:
   ```rust
   let template = FFmpegCommand::new(ffmpeg.clone())
       .input(input_path);
       
   // Create multiple commands from the template
   let mut cmd1 = template.clone();
   cmd1.output(output_path1).execute()?;
       
   let mut cmd2 = template.clone();
   cmd2.output(output_path2).execute()?;
   ```

4. **Handling String Conversions**
   
   For parameters that require string conversion, create bindings to extend lifetimes:
   ```rust
   let sample_rate_str = sample_rate.to_string();
   let channels_str = channels.to_string();
   
   cmd.output_options(&[
       "-ar", &sample_rate_str, 
       "-ac", &channels_str
   ]);
   ```

These practices ensure memory safety while maintaining an ergonomic API for building FFmpeg commands.

### Operation Interface (operations/mod.rs)

The operation interface defines how operations are implemented:

```rust
pub trait Operation {
    /// Validate operation parameters
    fn validate(&self) -> Result<()>;
    
    /// Generate FFmpeg command for the operation
    fn generate_command(&self, info: &MediaInfo) -> Result<FFmpegCommand>;
    
    /// Get estimated duration for progress reporting
    fn get_estimated_duration(&self, info: &MediaInfo) -> Option<Duration>;
    
    /// Get description of the operation
    fn get_description(&self) -> String;
}

pub struct OperationContext {
    pub input_info: Option<MediaInfo>,
    pub temp_dir: PathBuf,
}
```

### Media Information (media_info.rs)

The media information structure holds metadata about media files:

```rust
pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub dimensions: Option<(u32, u32)>,
    pub codec: Option<String>,
    pub frame_rate: Option<f64>,
    pub bitrate: Option<u64>,
}

struct MediaInfoParser;

impl MediaInfoParser {
    /// Parse FFmpeg output for media information
    pub fn parse(output: &str) -> Result<MediaInfo> {
        let mut info = MediaInfo {
            duration: None,
            dimensions: None,
            codec: None,
            frame_rate: None,
            bitrate: None,
        };
        
        // Duration
        if let Some(duration) = Self::parse_duration(output) {
            info.duration = Some(duration);
        }
        
        // Dimensions
        if let Some(dimensions) = Self::parse_dimensions(output) {
            info.dimensions = Some(dimensions);
        }
        
        // Codec
        if let Some(codec) = Self::parse_codec(output) {
            info.codec = Some(codec);
        }
        
        // Frame rate
        if let Some(frame_rate) = Self::parse_frame_rate(output) {
            info.frame_rate = Some(frame_rate);
        }
        
        // Bitrate
        if let Some(bitrate) = Self::parse_bitrate(output) {
            info.bitrate = Some(bitrate);
        }
        
        Ok(info)
    }
    
    // Helper methods for parsing different aspects of media info
    fn parse_duration(output: &str) -> Option<Duration> {
        // Implementation of duration parsing
        None
    }
    
    fn parse_dimensions(output: &str) -> Option<(u32, u32)> {
        // Implementation of dimensions parsing
        None
    }
    
    fn parse_codec(output: &str) -> Option<String> {
        // Implementation of codec parsing
        None
    }
    
    fn parse_frame_rate(output: &str) -> Option<f64> {
        // Implementation of frame rate parsing
        None
    }
    
    fn parse_bitrate(output: &str) -> Option<u64> {
        // Implementation of bitrate parsing
        None
    }
}
```

### Processing Pipeline (pipeline.rs)

The processing pipeline executes operations:

```rust
pub struct ProcessingPipeline {
    ffmpeg: FFmpegWrapper,
    config: AppConfig,
}

impl ProcessingPipeline {
    /// Create a new processing pipeline
    pub fn new(config: AppConfig) -> Result<Self> {
        let ffmpeg = FFmpegWrapper::new(
            config.ffmpeg_path.clone(),
            config.temp_directory.clone(),
        );
        
        Ok(Self {
            ffmpeg,
            config,
        })
    }
    
    /// Execute an operation
    pub fn execute(&self, operation: Box<dyn Operation>, progress: Option<Box<dyn ProgressReporter>>) -> Result<()> {
        // Validate operation
        operation.validate()?;
        
        // Get input file information if available
        let input_info = self.get_input_info(&operation)?;
        
        // Generate command
        let command = operation.generate_command(&input_info)?;
        
        // Execute command
        self.ffmpeg.run_command(command, progress)
    }
    
    /// Get media information for a file
    pub fn get_media_info(&self, path: &Path) -> Result<MediaInfo> {
        self.ffmpeg.get_media_info(path)
    }
    
    /// Get input information for an operation
    fn get_input_info(&self, operation: &Box<dyn Operation>) -> Result<MediaInfo> {
        // Implementation depends on operation type
        // This is a simplified version
        let dummy_info = MediaInfo {
            duration: Some(Duration::from_secs(60)),
            dimensions: Some((1920, 1080)),
            codec: Some("h264".to_string()),
            frame_rate: Some(30.0),
            bitrate: Some(5000000),
        };
        
        Ok(dummy_info)
    }
}
```

## Operation Implementations

### Trim Operation (operations/trim.rs)

The trim operation cuts a portion of a video:

```rust
pub struct TrimOperation {
    input_path: PathBuf,
    output_path: PathBuf,
    start_time: Option<TimePosition>,
    end_time: Option<TimePosition>,
    recompress: bool,
}

impl TrimOperation {
    /// Create a new trim operation
    pub fn new(
        input_path: &Path,
        output_path: &Path,
        start_time: Option<TimePosition>,
        end_time: Option<TimePosition>,
        recompress: bool,
    ) -> Self {
        Self {
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            start_time,
            end_time,
            recompress,
        }
    }
}

impl Operation for TrimOperation {
    fn validate(&self) -> Result<()> {
        // Check if input file exists
        if !self.input_path.exists() {
            return Err(Error::InvalidArgument(format!(
                "Input file does not exist: {}",
                self.input_path.display()
            )));
        }
        
        // Validate start and end times
        if let Some(start) = &self.start_time {
            if start.to_seconds() < 0.0 {
                return Err(Error::InvalidArgument("Start time cannot be negative".to_string()));
            }
        }
        
        if let Some(start) = &self.start_time {
            if let Some(end) = &self.end_time {
                if end.to_seconds() <= start.to_seconds() {
                    return Err(Error::InvalidArgument(
                        "End time must be greater than start time".to_string()
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    fn generate_command(&self, info: &MediaInfo) -> Result<FFmpegCommand> {
        let mut command = FFmpegCommand::new();
        
        // Add input file
        let mut input_options = Vec::new();
        if let Some(start) = &self.start_time {
            input_options.push("-ss".to_string());
            input_options.push(start.to_string(TimeFormat::Seconds));
        }
        command = command.input(&self.input_path, input_options);
        
        // Add output file
        let mut output_options = Vec::new();
        
        // Handle duration
        if let Some(end) = &self.end_time {
            if let Some(start) = &self.start_time {
                let duration = end.to_seconds() - start.to_seconds();
                output_options.push("-t".to_string());
                output_options.push(duration.to_string());
            } else {
                output_options.push("-to".to_string());
                output_options.push(end.to_string(TimeFormat::Seconds));
            }
        }
        
        // Copy codecs unless recompression is requested
        if !self.recompress {
            output_options.push("-c".to_string());
            output_options.push("copy".to_string());
        }
        
        command = command.output(&self.output_path, output_options);
        
        Ok(command)
    }
    
    fn get_estimated_duration(&self, info: &MediaInfo) -> Option<Duration> {
        // Calculate estimated duration based on input info and trim points
        let total_duration = info.duration?;
        
        let start_seconds = self.start_time
            .map(|t| t.to_seconds())
            .unwrap_or(0.0);
            
        let end_seconds = self.end_time
            .map(|t| t.to_seconds())
            .unwrap_or(total_duration.as_secs_f64());
            
        let duration_seconds = end_seconds - start_seconds;
        if duration_seconds <= 0.0 {
            return None;
        }
        
        Some(Duration::from_secs_f64(duration_seconds))
    }
    
    fn get_description(&self) -> String {
        let start_str = self.start_time
            .map(|t| t.to_string(TimeFormat::HHMMSS))
            .unwrap_or_else(|| "start".to_string());
            
        let end_str = self.end_time
            .map(|t| t.to_string(TimeFormat::HHMMSS))
            .unwrap_or_else(|| "end".to_string());
            
        format!("Trimming video from {} to {}", start_str, end_str)
    }
}
```

### Concatenation Operation (operations/concat.rs)

The concatenation operation combines multiple videos:

```rust
pub struct ConcatOperation {
    input_paths: Vec<PathBuf>,
    output_path: PathBuf,
    recompress: bool,
}

impl ConcatOperation {
    pub fn new(input_paths: Vec<PathBuf>, output_path: PathBuf, recompress: bool) -> Self {
        Self {
            input_paths,
            output_path,
            recompress,
        }
    }
}

impl Operation for ConcatOperation {
    fn validate(&self) -> Result<()> {
        // Check if we have at least two input files
        if self.input_paths.len() < 2 {
            return Err(Error::InvalidArgument(
                "At least two input files are required for concatenation".to_string()
            ));
        }
        
        // Check if all input files exist
        for path in &self.input_paths {
            if !path.exists() {
                return Err(Error::InvalidArgument(format!(
                    "Input file does not exist: {}",
                    path.display()
                )));
            }
        }
        
        Ok(())
    }
    
    fn generate_command(&self, info: &MediaInfo) -> Result<FFmpegCommand> {
        let mut command = FFmpegCommand::new();
        
        if self.recompress {
            // Concatenation with re-encoding
            for path in &self.input_paths {
                command = command.input(path, Vec::new());
            }
            
            // Create filter complex for concatenation
            let mut filter = String::new();
            for i in 0..self.input_paths.len() {
                filter.push_str(&format!("[{}:v:0][{}:a:0]", i, i));
            }
            filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", self.input_paths.len()));
            
            command = command.filter_complex(&filter);
            
            // Set up output options
            let output_options = vec![
                "-map".to_string(),
                "[outv]".to_string(),
                "-map".to_string(),
                "[outa]".to_string(),
            ];
            
            command = command.output(&self.output_path, output_options);
        } else {
            // Concatenation with stream copying (faster but requires same codecs)
            
            // Create a temporary concatenation file
            let temp_file = tempfile::NamedTempFile::new()?;
            let temp_path = temp_file.path();
            
            // Write the concat file
            let mut concat_content = String::new();
            for path in &self.input_paths {
                concat_content.push_str(&format!("file '{}'\n", path.display()));
            }
            fs::write(temp_path, concat_content)?;
            
            // Set up concat demuxer
            let input_options = vec![
                "-f".to_string(),
                "concat".to_string(),
                "-safe".to_string(),
                "0".to_string(),
            ];
            
            command = command.input(temp_path, input_options);
            
            // Set up output options for stream copying
            let output_options = vec![
                "-c".to_string(),
                "copy".to_string(),
            ];
            
            command = command.output(&self.output_path, output_options);
        }
        
        Ok(command)
    }
    
    fn get_estimated_duration(&self, info: &MediaInfo) -> Option<Duration> {
        // Simplified estimation - in reality would need to check each file
        if self.input_paths.is_empty() {
            return None;
        }
        
        // Just return the duration of the first file multiplied by count as estimate
        info.duration.map(|d| Duration::from_secs((d.as_secs() * self.input_paths.len() as u64)))
    }
    
    fn get_description(&self) -> String {
        format!("Concatenating {} videos", self.input_paths.len())
    }
}
```

## Implementation Details

### Progress Reporting

The processing module provides detailed progress reporting:

```rust
struct FFmpegProgress {
    position: u64,
    total: u64,
    message: String,
}

/// Parse FFmpeg progress information from output
fn parse_progress(line: &str) -> Option<FFmpegProgress> {
    // Look for time= in the output
    if let Some(time_pos) = line.find("time=") {
        let time_part = &line[time_pos + 5..];
        
        // Parse the timestamp
        if let Some(end_pos) = time_part.find(' ') {
            let time_str = &time_part[..end_pos];
            
            // Parse the time string (format: HH:MM:SS.MS)
            if let Ok(position) = parse_ffmpeg_time(time_str) {
                return Some(FFmpegProgress {
                    position,
                    total: 100, // Will be updated with actual total if known
                    message: format!("Processed {}s", position as f64 / 1000.0),
                });
            }
        }
    }
    
    None
}

/// Parse FFmpeg time string to milliseconds
fn parse_ffmpeg_time(time_str: &str) -> Result<u64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    
    if parts.len() != 3 {
        return Err(Error::FFmpeg(format!("Invalid time format: {}", time_str)));
    }
    
    let hours: u64 = parts[0].parse()?;
    let minutes: u64 = parts[1].parse()?;
    let seconds: f64 = parts[2].parse()?;
    
    let milliseconds = (hours * 3600 * 1000) + (minutes * 60 * 1000) + (seconds * 1000.0) as u64;
    
    Ok(milliseconds)
}
```

### Filter Graph Creation

For complex video operations, the module builds filter graphs:

```rust
pub struct FilterGraph {
    nodes: Vec<FilterNode>,
    edges: Vec<FilterEdge>,
}

struct FilterNode {
    id: String,
    filter: String,
}

struct FilterEdge {
    from: String,
    to: String,
}

impl FilterGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
    
    pub fn add_node(&mut self, id: &str, filter: &str) {
        self.nodes.push(FilterNode {
            id: id.to_string(),
            filter: filter.to_string(),
        });
    }
    
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push(FilterEdge {
            from: from.to_string(),
            to: to.to_string(),
        });
    }
    
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        
        // Add nodes
        for node in &self.nodes {
            result.push_str(&format!("{}={};", node.id, node.filter));
        }
        
        // Add edges
        for edge in &self.edges {
            result.push_str(&format!("{}>{};", edge.from, edge.to));
        }
        
        result
    }
}
```

### Temporary File Management

The module handles temporary files needed for complex operations:

```rust
struct TempFileManager {
    temp_dir: PathBuf,
    files: Vec<PathBuf>,
}

impl TempFileManager {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self {
            temp_dir,
            files: Vec::new(),
        }
    }
    
    pub fn create_temp_file(&mut self, prefix: &str, extension: &str) -> Result<PathBuf> {
        let file_name = format!("{}-{}.{}", prefix, Uuid::new_v4(), extension);
        let path = self.temp_dir.join(file_name);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Remember the file for cleanup
        self.files.push(path.clone());
        
        Ok(path)
    }
}

impl Drop for TempFileManager {
    fn drop(&mut self) {
        // Clean up all temporary files
        for file in &self.files {
            let _ = fs::remove_file(file);
        }
    }
}
```

## Integration with Other Modules

### Core Module Integration

The Processing module integrates with the Core module for configuration and logging:

```rust
impl ProcessingPipeline {
    /// Create a new processing pipeline from context
    pub fn from_context(context: &Context) -> Result<Self> {
        Self::new(context.config.clone())
    }
    
    /// Log operation status
    fn log_operation_status(&self, operation: &Box<dyn Operation>, result: &Result<()>, context: &Context) {
        match result {
            Ok(_) => {
                context.logger.info(&format!("Operation completed: {}", operation.get_description()));
            }
            Err(e) => {
                context.logger.error(&format!("Operation failed: {} - {}", operation.get_description(), e));
            }
        }
    }
}
```

### CLI Module Integration

The Processing module integrates with the CLI module for command execution:

```rust
pub fn execute_trim(args: &TrimArgs, context: &Context) -> Result<()> {
    let pipeline = ProcessingPipeline::from_context(context)?;
    
    // Parse time arguments
    let start_time = args.start.as_ref().map(|s| TimePosition::from_string(s))
        .transpose()?;
    let end_time = args.end.as_ref().map(|s| TimePosition::from_string(s))
        .transpose()?;
    
    // Create operation
    let operation = TrimOperation::new(
        &PathBuf::from(&args.input),
        &PathBuf::from(&args.output),
        start_time,
        end_time,
        args.recompress,
    );
    
    // Create progress reporter
    let progress = context.create_progress_bar(
        &operation.get_description(),
        None, // We'll get the duration during execution
    );
    
    // Execute operation
    pipeline.execute(Box::new(operation), Some(progress))
}
```

### Project Module Integration

The Processing module integrates with the Project module for rendering projects:

```rust
pub fn render_project(project: &Project, output_path: &Path, context: &Context) -> Result<()> {
    let pipeline = ProcessingPipeline::from_context(context)?;
    
    // Implementation of project rendering logic
    // This would involve creating a complex operation or sequence of operations
    // based on the project timeline
    
    Ok(())
}
```

## Testing Strategy

### Unit Testing

Test individual components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffmpeg_command_building() {
        let command = FFmpegCommand::new()
            .input(Path::new("input.mp4"), vec!["-ss".to_string(), "10".to_string()])
            .output(Path::new("output.mp4"), vec!["-t".to_string(), "30".to_string()]);
            
        let args = command.build();
        
        assert!(args.contains(&"-ss".to_string()));
        assert!(args.contains(&"10".to_string()));
        assert!(args.contains(&"-t".to_string()));
        assert!(args.contains(&"30".to_string()));
        assert!(args.contains(&"input.mp4".to_string()));
        assert!(args.contains(&"output.mp4".to_string()));
    }
    
    #[test]
    fn test_trim_operation_validation() {
        // Valid operation
        let valid_op = TrimOperation::new(
            &PathBuf::from("test_fixtures/sample.mp4"),
            &PathBuf::from("output.mp4"),
            Some(TimePosition::from_seconds(10.0)),
            Some(TimePosition::from_seconds(20.0)),
            false,
        );
        
        // Mock that the file exists
        // In a real test, we'd use a test fixture or mock
        
        assert!(valid_op.validate().is_ok());
        
        // Invalid operation (end before start)
        let invalid_op = TrimOperation::new(
            &PathBuf::from("test_fixtures/sample.mp4"),
            &PathBuf::from("output.mp4"),
            Some(TimePosition::from_seconds(20.0)),
            Some(TimePosition::from_seconds(10.0)),
            false,
        );
        
        assert!(invalid_op.validate().is_err());
    }
    
    #[test]
    fn test_parse_ffmpeg_time() {
        let time = parse_ffmpeg_time("01:23:45.67").unwrap();
        assert_eq!(time, 5025670); // 1h 23m 45.67s in milliseconds
        
        let invalid = parse_ffmpeg_time("not a time");
        assert!(invalid.is_err());
    }
}
```

### Integration Testing

Test the interaction between components:

```rust
#[test]
fn test_trim_operation_execution() {
    // Create a test configuration
    let config = AppConfig {
        ffmpeg_path: PathBuf::from("/usr/bin/ffmpeg"), // Adjust for your system
        temp_directory: PathBuf::from("./temp"),
        default_format: "mp4".to_string(),
        threading: ThreadingConfig::default(),
        logging: LoggingConfig::default(),
    };
    
    // Create pipeline
    let pipeline = ProcessingPipeline::new(config).unwrap();
    
    // Create operation
    let operation = TrimOperation::new(
        &PathBuf::from("test_fixtures/sample.mp4"),
        &PathBuf::from("test_output.mp4"),
        Some(TimePosition::from_seconds(1.0)),
        Some(TimePosition::from_seconds(3.0)),
        false,
    );
    
    // Execute operation
    let result = pipeline.execute(Box::new(operation), None);
    assert!(result.is_ok());
    
    // Verify output file exists
    assert!(PathBuf::from("test_output.mp4").exists());
    
    // Verify output duration
    let info = pipeline.get_media_info(&PathBuf::from("test_output.mp4")).unwrap();
    assert!(info.duration.unwrap().as_secs_f64() - 2.0 < 0.1);
    
    // Clean up
    let _ = fs::remove_file("test_output.mp4");
}
```

This detailed module implementation guide provides a comprehensive blueprint for implementing the Processing module of the edv application, covering structure, key components, implementation details, and testing strategy. 