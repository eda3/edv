# edv - Subtitle Module Implementation

This document provides detailed implementation guidelines for the Subtitle module of the edv application.

## Overview

The Subtitle module provides comprehensive functionality for working with subtitle files, including parsing, editing, formatting, and styling. It serves as the foundation for all subtitle-related operations within the edv application, enabling users to manipulate subtitle content, timing, and appearance.

## Structure

```
src/subtitle/
├── mod.rs      // Module exports and core definitions
├── model.rs    // Subtitle data structures
├── parser.rs   // Subtitle parsing logic
├── format.rs   // Format detection and conversion
├── editor.rs   // Subtitle editing functionality
├── style.rs    // Text styling definitions
└── error.rs    // Error types and handling
```

## Key Components

### Core Module Structure (mod.rs)

The main module file exports the public API and defines core types:

```rust
pub use self::editor::{ShiftBuilder, SubtitleEditor};
pub use self::error::{Error, Result};
pub use self::format::SubtitleFormat;
pub use self::model::{Subtitle, SubtitleTrack};
pub use self::style::TextStyle;

pub mod editor;
pub mod error;
pub mod format;
pub mod model;
pub mod parser;
pub mod style;

/// Represents subtitle encoding options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleEncoding {
    /// UTF-8 encoding
    Utf8,
    /// UTF-16 encoding (common in some subtitle files)
    Utf16,
    /// Latin-1 encoding (ISO-8859-1, common in older SRT files)
    Latin1,
    /// Automatically detect encoding (best effort)
    Auto,
}

/// Settings for subtitle rendering.
#[derive(Debug, Clone)]
pub struct RenderSettings {
    /// Font name to use
    pub font: String,
    /// Font size in points
    pub font_size: u32,
    /// Font color as #RRGGBB or #RRGGBBAA
    pub color: String,
    /// Outline color as #RRGGBB or #RRGGBBAA
    pub outline_color: String,
    /// Outline width in pixels
    pub outline_width: f32,
    /// Background color as #RRGGBB or #RRGGBBAA (empty for transparent)
    pub background: String,
    /// Vertical position (0.0 = top, 1.0 = bottom)
    pub position: f32,
    /// Enable drop shadow
    pub shadow: bool,
    /// Shadow color as #RRGGBB or #RRGGBBAA
    pub shadow_color: String,
    /// Shadow offset in pixels
    pub shadow_offset: f32,
}
```

### Subtitle Model (model.rs)

The model component defines the fundamental data structures for representing subtitles:

```rust
/// Represents a single subtitle entry with timing and text.
#[derive(Debug, Clone, PartialEq)]
pub struct Subtitle {
    /// Unique identifier for the subtitle
    id: String,
    /// Start time of the subtitle
    start_time: TimePosition,
    /// End time of the subtitle
    end_time: TimePosition,
    /// Text content of the subtitle (can contain multiple lines)
    text: String,
    /// Style information for the subtitle (optional)
    style: Option<TextStyle>,
}

/// A collection of subtitles.
#[derive(Debug, Clone, Default)]
pub struct SubtitleTrack {
    /// Collection of subtitles by ID
    subtitles: HashMap<String, Subtitle>,
    /// Ordered list of subtitle IDs for iteration
    order: Vec<String>,
}
```

The `Subtitle` struct provides methods for timing manipulation, format conversion, and overlap detection:

```rust
impl Subtitle {
    /// Creates a new subtitle.
    #[must_use]
    pub fn new(start_time: TimePosition, end_time: TimePosition, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            id: String::new(),
            start_time,
            end_time,
            text,
            style: None,
        }
    }
    
    /// Checks if this subtitle overlaps with another subtitle.
    #[must_use]
    pub fn overlaps_with(&self, other: &Self) -> bool {
        let self_start = self.start_time.as_seconds();
        let self_end = self.end_time.as_seconds();
        let other_start = other.start_time.as_seconds();
        let other_end = other.end_time.as_seconds();

        // Check if one subtitle starts before the other ends
        self_start < other_end && other_start < self_end
    }
    
    /// Shifts the subtitle timing by the specified number of seconds.
    pub fn shift(&mut self, seconds: f64) {
        let start_seconds = self.start_time.as_seconds() + seconds;
        let end_seconds = self.end_time.as_seconds() + seconds;

        // Ensure times don't go below zero
        let start_seconds = start_seconds.max(0.0);
        let end_seconds = end_seconds.max(start_seconds + 0.1);

        self.start_time = TimePosition::from_seconds(start_seconds);
        self.end_time = TimePosition::from_seconds(end_seconds);
    }
}
```

The `SubtitleTrack` provides collection management methods:

```rust
impl SubtitleTrack {
    /// Creates a new, empty subtitle track.
    #[must_use]
    pub fn new() -> Self {
        Self {
            subtitles: HashMap::new(),
            order: Vec::new(),
        }
    }
    
    /// Adds a subtitle to the track.
    pub fn add_subtitle(&mut self, subtitle: Subtitle) {
        let id = subtitle.get_id().to_string();
        // If ID is empty, generate one
        let id = if id.is_empty() {
            let new_id = (self.subtitles.len() + 1).to_string();
            let mut subtitle = subtitle;
            subtitle.set_id(new_id.clone());
            new_id
        } else {
            id
        };

        self.subtitles.insert(id.clone(), subtitle);
        if !self.order.contains(&id) {
            self.order.push(id);
        }
    }
    
    /// Finds overlapping subtitles in the track.
    #[must_use]
    pub fn find_overlaps(&self) -> Vec<(String, String)> {
        let mut overlaps = Vec::new();
        let subtitles = self.get_subtitles();
        
        for i in 0..subtitles.len() {
            for j in (i + 1)..subtitles.len() {
                if subtitles[i].overlaps_with(subtitles[j]) {
                    overlaps.push((
                        subtitles[i].get_id().to_string(),
                        subtitles[j].get_id().to_string(),
                    ));
                }
            }
        }
        
        overlaps
    }
}
```

### Format Handling (format.rs)

The format component handles subtitle format detection and time position parsing:

```rust
/// Supported subtitle formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleFormat {
    /// `SubRip` Text format (.srt)
    Srt,
    /// `WebVTT` format (.vtt)
    WebVtt,
    /// Advanced `SubStation` Alpha (.ass, .ssa)
    AdvancedSsa,
    /// `SubViewer` format (.sub)
    SubViewer,
    /// `MicroDVD` format (.sub)
    MicroDVD,
}

impl SubtitleFormat {
    /// Detects the subtitle format from a file extension.
    pub fn from_extension(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);

        match extension.as_deref() {
            Some("srt") => Ok(Self::Srt),
            Some("vtt") => Ok(Self::WebVtt),
            Some(ext) if ext == "ass" || ext == "ssa" => Ok(Self::AdvancedSsa),
            Some("sub") => Ok(Self::SubViewer), // Default to SubViewer
            _ => Err(Error::formatting_error(format!(
                "Unsupported subtitle extension: {extension:?}"
            ))),
        }
    }
    
    /// Attempts to detect the subtitle format from file content.
    pub fn detect_from_content(content: &str) -> Result<Self> {
        // Check for WebVTT signature
        if content.trim_start().starts_with("WEBVTT") {
            return Ok(Self::WebVtt);
        }

        // Check for ASS/SSA signature
        if content.trim_start().starts_with("[Script Info]") {
            return Ok(Self::AdvancedSsa);
        }

        // Additional format detection logic...
        
        // Default to SRT if we can't determine the format
        Err(Error::formatting_error(
            "Could not determine subtitle format from content",
        ))
    }
}

/// Represents a time position in a subtitle.
pub struct TimePosition {
    /// Hours component
    pub hours: u32,
    /// Minutes component
    pub minutes: u32,
    /// Seconds component
    pub seconds: u32,
    /// Milliseconds component
    pub milliseconds: u32,
}

impl TimePosition {
    /// Creates a new time position.
    #[must_use]
    pub fn new(hours: u32, minutes: u32, seconds: u32, milliseconds: u32) -> Self {
        // Normalize values
        let mut total_seconds = seconds + minutes * 60 + hours * 3600;
        let mut total_millis = milliseconds;

        // Handle millisecond overflow
        if total_millis >= 1000 {
            total_seconds += total_millis / 1000;
            total_millis %= 1000;
        }

        // Recalculate hours, minutes, seconds
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        Self {
            hours,
            minutes,
            seconds,
            milliseconds: total_millis,
        }
    }
    
    /// Creates a time position from seconds.
    #[must_use]
    pub fn from_seconds(total_seconds: f64) -> Self {
        let total_seconds = total_seconds.max(0.0);
        let whole_seconds = total_seconds.floor() as u32;
        let milliseconds = ((total_seconds - whole_seconds as f64) * 1000.0).round() as u32;

        let hours = whole_seconds / 3600;
        let minutes = (whole_seconds % 3600) / 60;
        let seconds = whole_seconds % 60;

        Self {
            hours,
            minutes,
            seconds,
            milliseconds,
        }
    }
}
```

### Subtitle Editor (editor.rs)

The editor component provides high-level functionality for manipulating subtitle tracks:

```rust
/// A builder for creating subtitles with time-shifted values.
#[derive(Debug)]
pub struct ShiftBuilder {
    /// Amount of time to shift in seconds (positive or negative)
    shift_seconds: f64,
    /// Only shift subtitles after this time
    after: Option<TimePosition>,
    /// Only shift subtitles before this time
    before: Option<TimePosition>,
    /// Only shift subtitles with these IDs
    ids: Vec<String>,
}

/// Provides editing capabilities for subtitle tracks.
#[derive(Debug)]
pub struct SubtitleEditor {
    /// The subtitle track being edited
    track: SubtitleTrack,
    /// The format the track should be saved as
    format: SubtitleFormat,
    /// Path to the file if loaded from disk
    file_path: Option<String>,
    /// Flag to track if the subtitle track has been modified
    modified: bool,
}

impl SubtitleEditor {
    /// Creates a new empty subtitle editor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            track: SubtitleTrack::new(),
            format: SubtitleFormat::Srt,
            file_path: None,
            modified: false,
        }
    }

    /// Loads a subtitle file into the editor.
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        // Implementation details...
    }
    
    /// Shift all subtitles by the specified amount.
    pub fn shift_subtitles(&mut self, shift: &ShiftBuilder) -> usize {
        let count = shift.apply(&mut self.track);
        if count > 0 {
            self.modified = true;
        }
        count
    }
    
    /// Merge adjacent subtitles that are close in time.
    pub fn merge_adjacent_subtitles(&mut self, max_gap: f64, same_style: bool) -> usize {
        // Implementation details...
    }
    
    /// Split a subtitle at the specified time.
    pub fn split_subtitle(&mut self, id: &str, split_time: TimePosition) -> Result<bool> {
        // Implementation details...
    }
    
    /// Fix overlapping subtitles using the specified strategy.
    pub fn fix_overlaps(&mut self, strategy: &str, min_gap: f64) -> usize {
        // Implementation details...
    }
}
```

### Style Handling (style.rs)

The style component defines the styling capabilities for subtitles:

```rust
/// Represents text alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    /// Left-aligned text
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
}

/// Represents a font weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    /// Normal font weight
    Normal,
    /// Bold font weight
    Bold,
}

/// Represents text styling information.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font family to use (e.g., "Arial")
    pub font_family: Option<String>,
    /// Font size in points or pixels
    pub font_size: Option<f32>,
    /// Font weight (normal or bold)
    pub font_weight: Option<FontWeight>,
    /// Whether the text is italic
    pub italic: Option<bool>,
    /// Whether the text is underlined
    pub underline: Option<bool>,
    /// Text color in #RRGGBB or #RRGGBBAA format
    pub color: Option<String>,
    /// Background color in #RRGGBB or #RRGGBBAA format
    pub background: Option<String>,
    /// Text alignment
    pub alignment: Option<TextAlignment>,
}
```

### Error Handling (error.rs)

The error component defines a comprehensive error handling system:

```rust
/// Result type for subtitle operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during subtitle processing operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Error when parsing subtitle files
    #[error("Failed to parse subtitle file: {reason}")]
    ParseError {
        /// Path to the subtitle file
        file: Option<PathBuf>,
        /// The reason for the parsing failure
        reason: String,
    },

    /// Error when an invalid subtitle format is provided
    #[error("Invalid subtitle format: {0}")]
    InvalidSubtitleFormat(String),

    /// Error when subtitle format is unknown
    #[error("Unknown subtitle format")]
    UnknownFormat,
    
    // Additional error variants...
}

impl Error {
    /// Creates a parse error with the given file and reason.
    #[must_use]
    pub fn parse_error(file: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::ParseError {
            file: Some(file.into()),
            reason: reason.into(),
        }
    }
    
    // Additional factory methods...
}
```

### Parser (parser.rs)

The parser component handles reading subtitle files in various formats:

```rust
/// Parses a subtitle file into a SubtitleTrack.
///
/// # Arguments
///
/// * `path` - Path to the subtitle file
/// * `format` - Optional format override (if not provided, will be detected from extension)
///
/// # Returns
///
/// A `SubtitleTrack` containing the parsed subtitles
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed
pub fn parse_subtitle_file<P: AsRef<Path>>(
    path: P,
    format: Option<SubtitleFormat>,
) -> Result<SubtitleTrack> {
    // Read the file content
    let path_ref = path.as_ref();
    let content = std::fs::read_to_string(path_ref)
        .map_err(|e| Error::parse_error(path_ref, format!("Failed to read file: {e}")))?;

    // Determine format if not specified
    let format = if let Some(fmt) = format {
        fmt
    } else {
        SubtitleFormat::from_extension(path_ref)?
    };

    // Parse based on format
    match format {
        SubtitleFormat::Srt => parse_srt(&content, path_ref),
        SubtitleFormat::WebVtt => parse_vtt(&content, path_ref),
        SubtitleFormat::AdvancedSsa => parse_ass(&content, path_ref),
        SubtitleFormat::SubViewer => parse_subviewer(&content, path_ref),
        SubtitleFormat::MicroDVD => parse_microdvd(&content, path_ref),
    }
}

// Format-specific parsing functions follow...
```

## Implementation Status

The Subtitle module has been implemented with the following features:

1. **Subtitle Data Model**:
   - Comprehensive `Subtitle` and `SubtitleTrack` structures ✅
   - Support for timing, text content, and styling ✅
   - Efficient subtitle lookup and management ✅

2. **Format Support**:
   - SubRip (SRT) format: fully implemented ✅
   - WebVTT format: fully implemented ✅
   - Advanced SubStation Alpha (ASS/SSA): parsing support ✅
   - SubViewer format: basic support ✅
   - MicroDVD format: basic support ✅

3. **Subtitle Editing**:
   - Timing adjustments (shift, scale) ✅
   - Content editing ✅
   - Style manipulation ✅
   - Merge and split operations ✅
   - Overlap detection and resolution ✅

4. **Parser and Writer**:
   - Format detection from file extension and content ✅
   - Robust parsing with error handling ✅
   - Format conversion ✅
   - Encoding detection and handling ✅

5. **Integration**:
   - FFmpeg integration for subtitle burning ✅
   - Project timeline integration ✅

## Current Limitations

1. **Advanced Styling**:
   - Limited support for complex styling in ASS/SSA format
   - Basic styling for WebVTT, minimal styling for SRT

2. **Performance Optimization**:
   - Large subtitle files may experience performance issues
   - Memory usage optimizations needed for very large files

3. **Format Support**:
   - Some less common subtitle formats are not yet supported
   - Limited support for bitmap-based subtitle formats

## Future Development

The following enhancements are planned for the Subtitle module:

1. **Enhanced Format Support**:
   - Improved ASS/SSA support with advanced styling
   - Additional subtitle formats (TTML, DFXP, etc.)
   - Support for DVD/Blu-ray subtitle formats

2. **Performance Improvements**:
   - Lazy loading for large subtitle files
   - Optimized memory usage for subtitle tracks
   - Faster parsing algorithms

3. **Advanced Editing**:
   - Subtitle synchronization with audio waveforms
   - Batch processing improvements
   - Advanced text formatting tools

4. **Integration**:
   - Enhanced timeline integration with preview capabilities
   - Machine learning-based subtitle generation
   - Speech recognition integration for automatic subtitling

## Usage Examples

### Basic Subtitle Editing

```rust
use edv::subtitle::editor::SubtitleEditor;
use edv::subtitle::format::{SubtitleFormat, TimePosition};
use edv::subtitle::model::Subtitle;

// Create a new subtitle editor
let mut editor = SubtitleEditor::new();

// Load a subtitle file
editor.load_file("subtitles.srt").unwrap();

// Add a new subtitle
let subtitle = Subtitle::new(
    TimePosition::from_seconds(10.0),
    TimePosition::from_seconds(15.0),
    "Hello, world!"
);
editor.add_subtitle(subtitle);

// Shift all subtitles by 2 seconds
use edv::subtitle::editor::ShiftBuilder;
let shift = ShiftBuilder::new(2.0);
editor.shift_subtitles(&shift);

// Save the modified subtitles
editor.save(Some("subtitles_modified.srt")).unwrap();
```

### Format Conversion

```rust
use edv::subtitle::editor::SubtitleEditor;
use edv::subtitle::format::SubtitleFormat;

// Load SRT file
let mut editor = SubtitleEditor::new();
editor.load_file("subtitles.srt").unwrap();

// Change format to WebVTT
editor.set_format(SubtitleFormat::WebVtt);

// Save as WebVTT
editor.save(Some("subtitles.vtt")).unwrap();
```

### Fixing Subtitle Overlaps

```rust
use edv::subtitle::editor::SubtitleEditor;

// Load subtitle file
let mut editor = SubtitleEditor::new();
editor.load_file("subtitles.srt").unwrap();

// Fix overlapping subtitles with 100ms minimum gap
// using the "shift" strategy
let fixed_count = editor.fix_overlaps("shift", 0.1);
println!("Fixed {} overlapping subtitles", fixed_count);

// Save the fixed subtitles
editor.save(None).unwrap();
``` 