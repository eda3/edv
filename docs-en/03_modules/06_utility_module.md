# edv - Utility Module Implementation

This document provides detailed implementation guidelines for the Utility module of the edv application.

## Overview

The Utility module provides common functionality and shared utilities used across the edv application. It serves as a foundation for many core operations, handling time code parsing and formatting, file operations, format detection, string manipulation, and various other general-purpose tasks that support the application's functionality.

## Structure

```
src/utility/
├── mod.rs                 // Module exports
├── time/                  // Time handling utilities
│   ├── mod.rs             // Time exports
│   ├── position.rs        // Time position implementation
│   ├── duration.rs        // Duration implementation
│   ├── format.rs          // Time format handling
│   └── timecode.rs        // Timecode utilities
├── file/                  // File handling utilities
│   ├── mod.rs             // File exports
│   ├── operations.rs      // File operations
│   ├── types.rs           // File type detection
│   └── temp.rs            // Temporary file management
├── format/                // Format handling
│   ├── mod.rs             // Format exports
│   ├── detect.rs          // Format detection
│   ├── convert.rs         // Format conversion
│   └── compatibility.rs   // Format compatibility
├── string/                // String utilities
│   ├── mod.rs             // String exports
│   ├── format.rs          // String formatting
│   └── parse.rs           // String parsing
└── error/                 // Error handling utilities
    ├── mod.rs             // Error exports
    └── context.rs         // Error context utilities
```

## Key Components

### Time Position (time/position.rs)

The time position implementation for representing points in time:

```rust
/// Represents a position in time for media editing
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TimePosition {
    /// Time in seconds (can include fractional seconds)
    seconds: f64,
}

impl TimePosition {
    /// Create a new time position from seconds
    pub fn from_seconds(seconds: f64) -> Self {
        if seconds < 0.0 {
            Self { seconds: 0.0 }
        } else {
            Self { seconds }
        }
    }
    
    /// Create a time position from hours, minutes, and seconds
    pub fn from_hms(hours: u32, minutes: u32, seconds: f64) -> Self {
        let total_seconds = (hours as f64) * 3600.0 + (minutes as f64) * 60.0 + seconds;
        Self::from_seconds(total_seconds)
    }
    
    /// Create a time position from a frame number and frame rate
    pub fn from_frames(frames: u64, frame_rate: f64) -> Self {
        let seconds = frames as f64 / frame_rate;
        Self::from_seconds(seconds)
    }
    
    /// Create a time position from a string in various formats
    pub fn from_string(time_str: &str) -> Result<Self, Error> {
        // Try HH:MM:SS.mmm format
        if let Some(captures) = TIME_REGEX.captures(time_str) {
            let hours = captures.get(1).map_or(0, |m| m.as_str().parse::<u32>().unwrap_or(0));
            let minutes = captures.get(2).map_or(0, |m| m.as_str().parse::<u32>().unwrap_or(0));
            let seconds = captures.get(3).map_or(0.0, |m| m.as_str().parse::<f64>().unwrap_or(0.0));
            
            return Ok(Self::from_hms(hours, minutes, seconds));
        }
        
        // Try seconds format
        if let Ok(seconds) = time_str.parse::<f64>() {
            return Ok(Self::from_seconds(seconds));
        }
        
        Err(Error::InvalidTimeFormat(time_str.to_string()))
    }
    
    /// Get the time in seconds
    pub fn as_seconds(&self) -> f64 {
        self.seconds
    }
    
    /// Get the time as frame number based on the given frame rate
    pub fn as_frames(&self, frame_rate: f64) -> u64 {
        (self.seconds * frame_rate).round() as u64
    }
    
    /// Format the time position according to the specified format
    pub fn format(&self, format: TimeFormat) -> String {
        match format {
            TimeFormat::HhMmSsMs => {
                let hours = (self.seconds / 3600.0).floor() as u32;
                let minutes = ((self.seconds - hours as f64 * 3600.0) / 60.0).floor() as u32;
                let seconds = self.seconds - hours as f64 * 3600.0 - minutes as f64 * 60.0;
                
                format!("{:02}:{:02}:{:06.3}", hours, minutes, seconds)
            },
            TimeFormat::Seconds => {
                format!("{:.3}", self.seconds)
            },
            TimeFormat::Frames(frame_rate) => {
                format!("{}", self.as_frames(frame_rate))
            },
        }
    }
}

impl Add<Duration> for TimePosition {
    type Output = TimePosition;
    
    fn add(self, rhs: Duration) -> Self::Output {
        TimePosition::from_seconds(self.seconds + rhs.as_seconds())
    }
}

impl Sub<Duration> for TimePosition {
    type Output = TimePosition;
    
    fn sub(self, rhs: Duration) -> Self::Output {
        let result = self.seconds - rhs.as_seconds();
        TimePosition::from_seconds(if result < 0.0 { 0.0 } else { result })
    }
}

impl Sub<TimePosition> for TimePosition {
    type Output = Duration;
    
    fn sub(self, rhs: TimePosition) -> Self::Output {
        let diff = self.seconds - rhs.seconds;
        Duration::from_seconds(if diff < 0.0 { 0.0 } else { diff })
    }
}
```

### Duration (time/duration.rs)

The duration implementation for representing time spans:

```rust
/// Represents a duration of time for media editing
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Duration {
    /// Duration in seconds (can include fractional seconds)
    seconds: f64,
}

impl Duration {
    /// Create a new duration from seconds
    pub fn from_seconds(seconds: f64) -> Self {
        if seconds < 0.0 {
            Self { seconds: 0.0 }
        } else {
            Self { seconds }
        }
    }
    
    /// Create a duration from hours, minutes, and seconds
    pub fn from_hms(hours: u32, minutes: u32, seconds: f64) -> Self {
        let total_seconds = (hours as f64) * 3600.0 + (minutes as f64) * 60.0 + seconds;
        Self::from_seconds(total_seconds)
    }
    
    /// Create a duration from a number of frames and frame rate
    pub fn from_frames(frames: u64, frame_rate: f64) -> Self {
        let seconds = frames as f64 / frame_rate;
        Self::from_seconds(seconds)
    }
    
    /// Create a duration from the difference between two time positions
    pub fn from_time_diff(start: TimePosition, end: TimePosition) -> Self {
        end - start
    }
    
    /// Create a duration from a string in various formats
    pub fn from_string(duration_str: &str) -> Result<Self, Error> {
        // Use same parsing logic as TimePosition
        TimePosition::from_string(duration_str).map(|tp| Self::from_seconds(tp.as_seconds()))
    }
    
    /// Get the duration in seconds
    pub fn as_seconds(&self) -> f64 {
        self.seconds
    }
    
    /// Get the duration as frame count based on the given frame rate
    pub fn as_frames(&self, frame_rate: f64) -> u64 {
        (self.seconds * frame_rate).round() as u64
    }
    
    /// Format the duration according to the specified format
    pub fn format(&self, format: TimeFormat) -> String {
        // Use same formatting logic as TimePosition
        TimePosition::from_seconds(self.seconds).format(format)
    }
}

impl Add for Duration {
    type Output = Duration;
    
    fn add(self, rhs: Self) -> Self::Output {
        Duration::from_seconds(self.seconds + rhs.seconds)
    }
}

impl Sub for Duration {
    type Output = Duration;
    
    fn sub(self, rhs: Self) -> Self::Output {
        let result = self.seconds - rhs.seconds;
        Duration::from_seconds(if result < 0.0 { 0.0 } else { result })
    }
}

impl Mul<f64> for Duration {
    type Output = Duration;
    
    fn mul(self, rhs: f64) -> Self::Output {
        Duration::from_seconds(self.seconds * rhs)
    }
}

impl Div<f64> for Duration {
    type Output = Duration;
    
    fn div(self, rhs: f64) -> Self::Output {
        if rhs == 0.0 {
            Duration::from_seconds(0.0)
        } else {
            Duration::from_seconds(self.seconds / rhs)
        }
    }
}
```

### File Operations (file/operations.rs)

Utilities for file system operations:

```rust
/// File operation utilities
pub struct FileUtils;

impl FileUtils {
    /// Ensure a directory exists, creating it if necessary
    pub fn ensure_directory_exists(path: &Path) -> Result<(), std::io::Error> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        } else if !path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Path exists but is not a directory: {}", path.display()),
            ));
        }
        
        Ok(())
    }
    
    /// Get a path for a temporary file with the given prefix and extension
    pub fn get_temporary_path(temp_dir: &Path, prefix: &str, extension: &str) -> PathBuf {
        let uuid = Uuid::new_v4();
        let filename = format!("{}_{}.{}", prefix, uuid, extension);
        
        temp_dir.join(filename)
    }
    
    /// Check if a file exists and is readable
    pub fn is_file_readable(path: &Path) -> bool {
        use std::fs::File;
        
        if !path.exists() || !path.is_file() {
            return false;
        }
        
        match File::open(path) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    /// Copy a file with progress reporting
    pub fn copy_file_with_progress<P: ProgressReporter>(
        source: &Path,
        destination: &Path,
        progress: &P,
    ) -> Result<(), std::io::Error> {
        use std::fs::File;
        use std::io::{Read, Write};
        
        const BUFFER_SIZE: usize = 64 * 1024; // 64 KB buffer
        
        let mut source_file = File::open(source)?;
        let metadata = source_file.metadata()?;
        let total_size = metadata.len();
        
        let mut dest_file = File::create(destination)?;
        let mut buffer = [0; BUFFER_SIZE];
        let mut bytes_copied = 0u64;
        
        loop {
            let bytes_read = match source_file.read(&mut buffer) {
                Ok(0) => break, // End of file
                Ok(n) => n,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            
            dest_file.write_all(&buffer[0..bytes_read])?;
            bytes_copied += bytes_read as u64;
            
            // Report progress
            if total_size > 0 {
                let percentage = (bytes_copied as f64 / total_size as f64) * 100.0;
                progress.update(percentage as u32);
            }
        }
        
        // Ensure all data is written
        dest_file.flush()?;
        
        Ok(())
    }
    
    /// Get the file size in bytes
    pub fn get_file_size(path: &Path) -> Result<u64, std::io::Error> {
        let metadata = std::fs::metadata(path)?;
        Ok(metadata.len())
    }
    
    /// Check if a path is a video file based on extension
    pub fn is_video_file(path: &Path) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => {
                let ext = ext.to_lowercase();
                VIDEO_EXTENSIONS.contains(&ext.as_str())
            },
            None => false,
        }
    }
    
    /// Check if a path is an audio file based on extension
    pub fn is_audio_file(path: &Path) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => {
                let ext = ext.to_lowercase();
                AUDIO_EXTENSIONS.contains(&ext.as_str())
            },
            None => false,
        }
    }
}

/// Known video file extensions
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp", "ts", "mts",
];

/// Known audio file extensions
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "aac", "flac", "ogg", "m4a", "wma", "aiff", "alac",
];
```

### Temporary File Management (file/temp.rs)

Management of temporary files:

```rust
/// Manages temporary files and ensures cleanup
pub struct TempFileManager {
    /// Directory for temporary files
    temp_dir: PathBuf,
    /// List of created temporary files
    files: Vec<PathBuf>,
}

impl TempFileManager {
    /// Create a new temporary file manager
    pub fn new(temp_dir: PathBuf) -> Result<Self, std::io::Error> {
        // Ensure temp directory exists
        FileUtils::ensure_directory_exists(&temp_dir)?;
        
        Ok(Self {
            temp_dir,
            files: Vec::new(),
        })
    }
    
    /// Create a new temporary file with the given prefix and extension
    pub fn create_temp_file(&mut self, prefix: &str, extension: &str) -> PathBuf {
        let path = FileUtils::get_temporary_path(&self.temp_dir, prefix, extension);
        self.files.push(path.clone());
        path
    }
    
    /// Register an existing file for cleanup
    pub fn register_file(&mut self, path: PathBuf) {
        self.files.push(path);
    }
    
    /// Clean up a specific temporary file
    pub fn cleanup_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(index) = self.files.iter().position(|p| p == path) {
            self.files.remove(index);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        
        Ok(())
    }
    
    /// Clean up all temporary files
    pub fn cleanup_all(&mut self) -> Result<(), std::io::Error> {
        let mut last_error = None;
        
        // Try to remove all files, collecting errors
        for path in self.files.drain(..) {
            if path.exists() {
                if let Err(e) = std::fs::remove_file(&path) {
                    last_error = Some(e);
                }
            }
        }
        
        // Return the last error if any
        if let Some(e) = last_error {
            Err(e)
        } else {
            Ok(())
        }
    }
}

impl Drop for TempFileManager {
    fn drop(&mut self) {
        // Try to clean up files on drop, ignore errors
        let _ = self.cleanup_all();
    }
}
```

### Format Detection (format/detect.rs)

Utilities for detecting media formats:

```rust
/// Utilities for media format detection
pub struct FormatDetector;

impl FormatDetector {
    /// Detect format based on file extension
    pub fn detect_from_extension(path: &Path) -> Option<MediaFormat> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| {
                let ext = ext.to_lowercase();
                match ext.as_str() {
                    "mp4" => Some(MediaFormat::Mp4),
                    "avi" => Some(MediaFormat::Avi),
                    "mkv" => Some(MediaFormat::Mkv),
                    "mov" => Some(MediaFormat::Mov),
                    "webm" => Some(MediaFormat::WebM),
                    "mp3" => Some(MediaFormat::Mp3),
                    "wav" => Some(MediaFormat::Wav),
                    "aac" => Some(MediaFormat::Aac),
                    "flac" => Some(MediaFormat::Flac),
                    // Other formats...
                    _ => None,
                }
            })
    }
    
    /// Detect format by analyzing file content
    pub fn detect_from_content(path: &Path, ffmpeg: &FFmpegWrapper) -> Result<MediaFormat, Error> {
        // Execute FFmpeg to get file info
        let info = ffmpeg.get_media_info(path)?;
        
        if let Some(format) = info.format {
            match format.as_str() {
                "mp4" => Ok(MediaFormat::Mp4),
                "avi" => Ok(MediaFormat::Avi),
                "matroska" => Ok(MediaFormat::Mkv),
                "mov" => Ok(MediaFormat::Mov),
                "webm" => Ok(MediaFormat::WebM),
                "mp3" => Ok(MediaFormat::Mp3),
                "wav" => Ok(MediaFormat::Wav),
                "aac" => Ok(MediaFormat::Aac),
                "flac" => Ok(MediaFormat::Flac),
                // Other formats...
                _ => Ok(MediaFormat::Unknown(format)),
            }
        } else {
            Err(Error::UnknownFormat)
        }
    }
    
    /// Get format mime type
    pub fn get_mime_type(format: MediaFormat) -> &'static str {
        match format {
            MediaFormat::Mp4 => "video/mp4",
            MediaFormat::Avi => "video/x-msvideo",
            MediaFormat::Mkv => "video/x-matroska",
            MediaFormat::Mov => "video/quicktime",
            MediaFormat::WebM => "video/webm",
            MediaFormat::Mp3 => "audio/mpeg",
            MediaFormat::Wav => "audio/wav",
            MediaFormat::Aac => "audio/aac",
            MediaFormat::Flac => "audio/flac",
            MediaFormat::Unknown(_) => "application/octet-stream",
            // Other formats...
        }
    }
    
    /// Check if a format is a video format
    pub fn is_video_format(format: MediaFormat) -> bool {
        matches!(format,
            MediaFormat::Mp4 | 
            MediaFormat::Avi | 
            MediaFormat::Mkv | 
            MediaFormat::Mov | 
            MediaFormat::WebM
            // Other video formats...
        )
    }
    
    /// Check if a format is an audio format
    pub fn is_audio_format(format: MediaFormat) -> bool {
        matches!(format,
            MediaFormat::Mp3 | 
            MediaFormat::Wav | 
            MediaFormat::Aac | 
            MediaFormat::Flac
            // Other audio formats...
        )
    }
}

/// Represents a media format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaFormat {
    /// MP4 video format
    Mp4,
    /// AVI video format
    Avi,
    /// MKV video format
    Mkv,
    /// MOV video format
    Mov,
    /// WebM video format
    WebM,
    /// MP3 audio format
    Mp3,
    /// WAV audio format
    Wav,
    /// AAC audio format
    Aac,
    /// FLAC audio format
    Flac,
    /// Unknown format with identifier
    Unknown(String),
    // Other formats...
}

impl Display for MediaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaFormat::Mp4 => write!(f, "MP4"),
            MediaFormat::Avi => write!(f, "AVI"),
            MediaFormat::Mkv => write!(f, "MKV"),
            MediaFormat::Mov => write!(f, "MOV"),
            MediaFormat::WebM => write!(f, "WebM"),
            MediaFormat::Mp3 => write!(f, "MP3"),
            MediaFormat::Wav => write!(f, "WAV"),
            MediaFormat::Aac => write!(f, "AAC"),
            MediaFormat::Flac => write!(f, "FLAC"),
            MediaFormat::Unknown(s) => write!(f, "Unknown ({})", s),
            // Other formats...
        }
    }
}
```

### Error Context (error/context.rs)

Error context utilities for better error reporting:

```rust
/// Extension trait for Result to add context to errors
pub trait ErrorContextExt<T, E> {
    /// Add context to an error
    fn with_context<C, F>(self, context: F) -> Result<T, ContextError<E>>
    where
        F: FnOnce() -> C,
        C: Display + Send + Sync + 'static;
}

impl<T, E: Error + Send + Sync + 'static> ErrorContextExt<T, E> for Result<T, E> {
    fn with_context<C, F>(self, context: F) -> Result<T, ContextError<E>>
    where
        F: FnOnce() -> C,
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|error| {
            let context = context();
            ContextError {
                context: context.to_string(),
                source: error,
            }
        })
    }
}

/// Error with context information
#[derive(Debug)]
pub struct ContextError<E> {
    /// Context message
    context: String,
    /// Source error
    source: E,
}

impl<E: Error + Send + Sync + 'static> Error for ContextError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl<E: Error> Display for ContextError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.context, self.source)
    }
}
```

## Key Interfaces

### Time Handling Interface

The time utilities provide interfaces for:

- **Time Position Management**: Create, manipulate, and format time positions
- **Duration Handling**: Create, manipulate, and format time durations
- **Time Conversion**: Convert between different time representations
- **Time Format Handling**: Format time values for display

### File Operations Interface

The file utilities provide interfaces for:

- **File Management**: Operations for managing files
- **Directory Management**: Operations for managing directories
- **Temporary File Handling**: Creation and cleanup of temporary files
- **Progress Tracking**: File operations with progress reporting

### Format Detection Interface

The format utilities provide interfaces for:

- **Format Detection**: Detect media formats from files
- **Format Information**: Get information about media formats
- **Format Validation**: Validate file formats
- **Format Conversion**: Convert between different formats

### Error Handling Interface

The error utilities provide interfaces for:

- **Error Context**: Add contextual information to errors
- **Error Formatting**: Format errors for display
- **Error Handling**: Common error handling operations

## Performance Considerations

- **Time Calculation Efficiency**: Optimize time calculations for performance
- **File I/O Optimization**: Efficient file reading and writing
- **Cache-Friendly Algorithms**: Design algorithms to be cache-friendly
- **Memory Management**: Minimize memory usage and allocations

## Future Enhancements

- **Extended Time Formats**: Support for more time formats
- **Advanced File Operations**: More sophisticated file operations
- **Format Conversion**: Enhanced format conversion capabilities
- **Parallel File Operations**: Support for parallel file operations
- **Stream Processing**: Stream-based file processing for large files

This modular Utility implementation provides the foundation for various operations across the edv application, ensuring consistency, efficiency, and reliability in common tasks while reducing code duplication and complexity. 