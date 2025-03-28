pub use self::editor::{ShiftBuilder, SubtitleEditor};
/// Subtitle processing module.
///
/// This module provides functionality for working with subtitle files,
/// including parsing, editing, and rendering subtitles in various formats.
///
/// # Example
///
/// ```
/// use edv::subtitle::format::SubtitleFormat;
/// use edv::subtitle::editor::SubtitleEditor;
///
/// // Create a new subtitle editor
/// let mut editor = SubtitleEditor::new();
/// editor.create_new(SubtitleFormat::Srt);
///
/// // Now you can add subtitles, edit them, etc.
/// ```
///
/// # Features
///
/// - Support for multiple subtitle formats (SRT, WebVTT)
/// - Parsing and writing subtitle files
/// - Editing subtitle content and timing
/// - Styling subtitle text
/// - Batch operations like shifting timing and fixing overlaps
// Re-export public types
pub use self::error::{Error, Result};
pub use self::format::SubtitleFormat;
pub use self::model::{Subtitle, SubtitleTrack};
pub use self::style::TextStyle;

// Define submodules
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

impl Default for SubtitleEncoding {
    fn default() -> Self {
        Self::Auto
    }
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

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            font: "Sans".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(),
            outline_color: "#000000".to_string(),
            outline_width: 1.0,
            background: String::new(),
            position: 0.9, // Near bottom
            shadow: true,
            shadow_color: "#000000AA".to_string(),
            shadow_offset: 2.0,
        }
    }
}

impl RenderSettings {
    /// Creates a new instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the font.
    #[must_use]
    pub fn font(mut self, font: impl Into<String>) -> Self {
        self.font = font.into();
        self
    }

    /// Sets the font size.
    #[must_use]
    pub fn font_size(mut self, size: u32) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the font color.
    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the outline color.
    #[must_use]
    pub fn outline_color(mut self, color: impl Into<String>) -> Self {
        self.outline_color = color.into();
        self
    }

    /// Sets the outline width.
    #[must_use]
    pub fn outline_width(mut self, width: f32) -> Self {
        self.outline_width = width.max(0.0);
        self
    }

    /// Sets the background color.
    #[must_use]
    pub fn background(mut self, color: impl Into<String>) -> Self {
        self.background = color.into();
        self
    }

    /// Sets the vertical position.
    #[must_use]
    pub fn position(mut self, position: f32) -> Self {
        self.position = position.clamp(0.0, 1.0);
        self
    }

    /// Enables or disables shadow.
    #[must_use]
    pub fn shadow(mut self, enable: bool) -> Self {
        self.shadow = enable;
        self
    }

    /// Sets the shadow color.
    #[must_use]
    pub fn shadow_color(mut self, color: impl Into<String>) -> Self {
        self.shadow_color = color.into();
        self
    }

    /// Sets the shadow offset.
    #[must_use]
    pub fn shadow_offset(mut self, offset: f32) -> Self {
        self.shadow_offset = offset.max(0.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subtitle::format::TimePosition;

    #[test]
    fn test_subtitle_basic_workflow() {
        // Create a subtitle
        let start = TimePosition::from_seconds(1.0);
        let end = TimePosition::from_seconds(4.0);
        let subtitle = Subtitle::new(start, end, "Hello, world!").with_id("1");

        // Create a track and add the subtitle
        let mut track = SubtitleTrack::new();
        track.add_subtitle(subtitle);

        // Check that the subtitle was added
        assert_eq!(track.len(), 1);
        assert!(track.get_subtitle("1").is_some());

        // Create an editor and use the track
        let mut editor = editor::SubtitleEditor::new();
        editor.get_track_mut().add_subtitle(
            Subtitle::new(
                TimePosition::from_seconds(5.0),
                TimePosition::from_seconds(8.0),
                "Second subtitle",
            )
            .with_id("2"),
        );

        // Check that the editor has the subtitle
        assert_eq!(editor.get_track().len(), 1);
        assert!(editor.get_subtitle("2").is_some());

        // Shift all subtitles by 1 second
        let shift = editor::ShiftBuilder::new(1.0);
        editor.shift_subtitles(shift);

        // Check that the subtitle was shifted
        let subtitle = editor.get_subtitle("2").unwrap();
        assert_eq!(subtitle.get_start().as_seconds(), 6.0);
        assert_eq!(subtitle.get_end().as_seconds(), 9.0);
    }
}
