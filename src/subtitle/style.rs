/// Subtitle text styling definitions.
///
/// This module defines the data structures and functions for styling subtitles,
/// including text formatting, colors, and positioning.
use std::fmt;

/// Horizontal alignment options for subtitle text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    /// Left-aligned text
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
}

impl Default for HorizontalAlign {
    fn default() -> Self {
        Self::Center
    }
}

impl fmt::Display for HorizontalAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        };
        write!(f, "{s}")
    }
}

/// Vertical alignment options for subtitle text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    /// Top-aligned text
    Top,
    /// Middle-aligned text
    Middle,
    /// Bottom-aligned text
    Bottom,
}

impl Default for VerticalAlign {
    fn default() -> Self {
        Self::Bottom
    }
}

impl fmt::Display for VerticalAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
        };
        write!(f, "{s}")
    }
}

/// Font style options for text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    /// Normal text
    Normal,
    /// Italic text
    Italic,
    /// Bold text
    Bold,
    /// Bold and italic text
    BoldItalic,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

impl FontStyle {
    /// Returns the CSS representation of the font style.
    #[must_use]
    pub fn as_css(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
            Self::Bold => "bold",
            Self::BoldItalic => "bold italic",
        }
    }
}

/// Represents the styling for subtitle text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font family for the text
    pub font_family: String,

    /// Font size in pixels
    pub font_size: u32,

    /// Font color as #RRGGBB hex string
    pub color: String,

    /// Background color as #RRGGBB hex string (empty for transparent)
    pub background: String,

    /// Font style (normal, italic, bold)
    pub font_style: FontStyle,

    /// Horizontal alignment
    pub horizontal_align: HorizontalAlign,

    /// Vertical alignment
    pub vertical_align: VerticalAlign,

    /// Outline color as #RRGGBB hex string (empty for no outline)
    pub outline_color: String,

    /// Outline width in pixels
    pub outline_width: f32,

    /// Custom position relative to the video frame (0.0-1.0 for x and y)
    pub position: Option<(f32, f32)>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "sans-serif".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(), // White
            background: String::new(),    // Transparent
            font_style: FontStyle::default(),
            horizontal_align: HorizontalAlign::default(),
            vertical_align: VerticalAlign::default(),
            outline_color: "#000000".to_string(), // Black
            outline_width: 1.0,
            position: None,
        }
    }
}

impl TextStyle {
    /// Creates a new text style with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the font family.
    #[must_use]
    pub fn font_family(mut self, font: impl Into<String>) -> Self {
        self.font_family = font.into();
        self
    }

    /// Sets the font size in pixels.
    #[must_use]
    pub fn font_size(mut self, size: u32) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the text color as a hex string (#RRGGBB).
    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the background color as a hex string (#RRGGBB).
    #[must_use]
    pub fn background(mut self, color: impl Into<String>) -> Self {
        self.background = color.into();
        self
    }

    /// Sets the font style.
    #[must_use]
    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font_style = style;
        self
    }

    /// Sets the horizontal alignment.
    #[must_use]
    pub fn horizontal_align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self
    }

    /// Sets the vertical alignment.
    #[must_use]
    pub fn vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Sets the outline color as a hex string (#RRGGBB).
    #[must_use]
    pub fn outline_color(mut self, color: impl Into<String>) -> Self {
        self.outline_color = color.into();
        self
    }

    /// Sets the outline width in pixels.
    #[must_use]
    pub fn outline_width(mut self, width: f32) -> Self {
        self.outline_width = width.max(0.0);
        self
    }

    /// Sets a custom position relative to the video frame.
    ///
    /// # Arguments
    ///
    /// * `x` - Horizontal position (0.0 = left, 1.0 = right)
    /// * `y` - Vertical position (0.0 = top, 1.0 = bottom)
    #[must_use]
    pub fn position(mut self, x: f32, y: f32) -> Self {
        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        self.position = Some((x, y));
        self
    }

    /// Clears the custom position, reverting to alignment-based positioning.
    #[must_use]
    pub fn clear_position(mut self) -> Self {
        self.position = None;
        self
    }

    /// Converts the style to a `WebVTT` cue setting string.
    ///
    /// # Returns
    ///
    /// A string with `WebVTT` cue settings
    #[must_use]
    pub fn to_vtt_string(&self) -> String {
        let mut settings = Vec::new();

        // Position and alignment
        if let Some((x, y)) = self.position {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let x_percent = (x * 100.0) as u32;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let y_percent = (y * 100.0) as u32;
            settings.push(format!("position:{x_percent}%"));
            settings.push(format!("line:{y_percent}%"));
        } else {
            // Use alignment settings
            settings.push(format!("align:{}", self.horizontal_align));

            match self.vertical_align {
                VerticalAlign::Top => settings.push("line:0%".to_string()),
                VerticalAlign::Middle => settings.push("line:50%".to_string()),
                VerticalAlign::Bottom => settings.push("line:100%".to_string()),
            }
        }

        // Convert to a VTT cue setting string
        settings.join(" ")
    }

    /// Converts the style to a CSS style string.
    ///
    /// # Returns
    ///
    /// A string with CSS style properties
    #[must_use]
    pub fn to_css_string(&self) -> String {
        let mut styles = Vec::new();

        // Font properties
        styles.push(format!("font-family: {}", self.font_family));
        styles.push(format!("font-size: {}px", self.font_size));
        styles.push(format!("font-style: {}", self.font_style.as_css()));

        // Colors
        styles.push(format!("color: {}", self.color));

        if !self.background.is_empty() {
            styles.push(format!("background-color: {}", self.background));
        }

        // Text outline
        if self.outline_width > 0.0 && !self.outline_color.is_empty() {
            styles.push(format!(
                "text-shadow: 0 0 {}px {}",
                self.outline_width, self.outline_color
            ));
        }

        // Text alignment
        styles.push(format!("text-align: {}", self.horizontal_align));

        // Convert to a CSS style string
        styles.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_defaults() {
        let style = TextStyle::default();
        assert_eq!(style.font_family, "sans-serif");
        assert_eq!(style.font_size, 24);
        assert_eq!(style.color, "#FFFFFF");
        assert_eq!(style.horizontal_align, HorizontalAlign::Center);
        assert_eq!(style.vertical_align, VerticalAlign::Bottom);
    }

    #[test]
    fn test_style_builder() {
        let style = TextStyle::new()
            .font_family("Arial")
            .font_size(32)
            .color("#FFCC00")
            .background("#000000")
            .font_style(FontStyle::Italic)
            .horizontal_align(HorizontalAlign::Left)
            .vertical_align(VerticalAlign::Top)
            .outline_color("#333333")
            .outline_width(2.0)
            .position(0.5, 0.8);

        assert_eq!(style.font_family, "Arial");
        assert_eq!(style.font_size, 32);
        assert_eq!(style.color, "#FFCC00");
        assert_eq!(style.background, "#000000");
        assert_eq!(style.font_style, FontStyle::Italic);
        assert_eq!(style.horizontal_align, HorizontalAlign::Left);
        assert_eq!(style.vertical_align, VerticalAlign::Top);
        assert_eq!(style.outline_color, "#333333");
        assert_eq!(style.outline_width, 2.0);
        assert_eq!(style.position, Some((0.5, 0.8)));

        let style = style.clear_position();
        assert_eq!(style.position, None);
    }

    #[test]
    fn test_vtt_string_generation() {
        let style = TextStyle::new()
            .horizontal_align(HorizontalAlign::Right)
            .vertical_align(VerticalAlign::Top);

        let vtt = style.to_vtt_string();
        assert!(vtt.contains("align:right"));
        assert!(vtt.contains("line:0%"));

        let style = TextStyle::new().position(0.25, 0.75);
        let vtt = style.to_vtt_string();
        assert!(vtt.contains("position:25%"));
        assert!(vtt.contains("line:75%"));
    }

    #[test]
    fn test_css_string_generation() {
        let style = TextStyle::new()
            .font_family("Arial")
            .font_size(32)
            .color("#FFCC00")
            .font_style(FontStyle::BoldItalic);

        let css = style.to_css_string();
        assert!(css.contains("font-family: Arial"));
        assert!(css.contains("font-size: 32px"));
        assert!(css.contains("color: #FFCC00"));
        assert!(css.contains("font-style: bold italic"));
    }
}
