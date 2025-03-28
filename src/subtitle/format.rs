/// Subtitle format definitions and utilities.
///
/// This module defines the supported subtitle formats and provides
/// utilities for format detection and conversion.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::subtitle::error::{Error, Result};

/// Supported subtitle formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleFormat {
    /// SubRip Text format (.srt)
    Srt,
    
    /// WebVTT format (.vtt)
    Vtt,
    
    /// WebVTT format (.vtt) - 別名
    WebVtt,
    
    /// Advanced SubStation Alpha (.ass, .ssa)
    Ass,
    
    /// Advanced SubStation Alpha (.ass, .ssa) - 別名
    AdvancedSsa,
    
    /// SubViewer format (.sub)
    SubViewer,
    
    /// MicroDVD format (.sub)
    MicroDVD,
}

impl SubtitleFormat {
    /// Returns the file extension for this subtitle format.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt | Self::WebVtt => "vtt",
            Self::Ass | Self::AdvancedSsa => "ass",
            Self::SubViewer => "sub",
            Self::MicroDVD => "sub",
        }
    }
    
    /// Returns the MIME type for this subtitle format.
    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Srt => "application/x-subrip",
            Self::Vtt | Self::WebVtt => "text/vtt",
            Self::Ass | Self::AdvancedSsa => "text/x-ssa",
            Self::SubViewer => "text/x-sub",
            Self::MicroDVD => "text/x-sub",
        }
    }
    
    /// Detects the subtitle format from a file extension.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the subtitle file
    ///
    /// # Returns
    ///
    /// The detected subtitle format or an error if the format is unsupported
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension is missing or not recognized
    pub fn from_extension(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
            
        match extension.as_deref() {
            Some("srt") => Ok(Self::Srt),
            Some("vtt") => Ok(Self::WebVtt),
            Some("ass") | Some("ssa") => Ok(Self::AdvancedSsa),
            Some("sub") => {
                // For .sub files, we'll need to look at the content later
                // to determine if it's SubViewer or MicroDVD
                // For now, default to SubViewer
                Ok(Self::SubViewer)
            }
            _ => Err(Error::format_error(
                format!("Unsupported subtitle extension: {:?}", extension)
            )),
        }
    }
    
    /// Attempts to detect the subtitle format from file content.
    ///
    /// # Arguments
    ///
    /// * `content` - Content of the subtitle file
    ///
    /// # Returns
    ///
    /// The detected subtitle format or an error if the format is unsupported
    ///
    /// # Errors
    ///
    /// Returns an error if the content format can't be determined
    pub fn detect_from_content(content: &str) -> Result<Self> {
        // Check for WebVTT signature
        if content.trim_start().starts_with("WEBVTT") {
            return Ok(Self::WebVtt);
        }
        
        // Check for ASS/SSA signature
        if content.trim_start().starts_with("[Script Info]") {
            return Ok(Self::AdvancedSsa);
        }
        
        // Check for MicroDVD format (starts with frame numbers in curly braces)
        if content.trim_start().starts_with('{') {
            let first_line = content.lines().next().unwrap_or_default();
            // MicroDVD format has {frame1}{frame2}Text
            if first_line.matches('{').count() >= 2 && first_line.matches('}').count() >= 2 {
                return Ok(Self::MicroDVD);
            }
        }
        
        // Check for SubRip format (starts with a number followed by timecodes)
        let lines: Vec<&str> = content.lines().take(4).collect();
        if lines.len() >= 3 {
            // First line should be a number
            if lines[0].trim().parse::<u32>().is_ok() {
                // Second line should contain a time range with -->
                if lines[1].contains("-->") {
                    return Ok(Self::Srt);
                }
            }
        }
        
        // Check for SubViewer format (starts with timecodes)
        if content.contains("[INFORMATION]") || content.lines().any(|line| line.contains(":")) {
            return Ok(Self::SubViewer);
        }
        
        // Default to SRT if we can't determine the format
        Err(Error::format_error("Could not determine subtitle format from content"))
    }
    
    /// Checks if a file extension is supported for subtitles.
    ///
    /// # Arguments
    ///
    /// * `extension` - File extension to check
    ///
    /// # Returns
    ///
    /// `true` if the extension is supported, `false` otherwise
    #[must_use]
    pub fn is_supported_extension(extension: &str) -> bool {
        matches!(
            extension.to_lowercase().as_str(),
            "srt" | "vtt" | "ass" | "ssa" | "sub"
        )
    }
}

impl fmt::Display for SubtitleFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Srt => "SubRip",
            Self::Vtt => "WebVTT",
            Self::WebVtt => "WebVTT",
            Self::Ass => "Advanced SubStation Alpha",
            Self::AdvancedSsa => "Advanced SubStation Alpha",
            Self::SubViewer => "SubViewer",
            Self::MicroDVD => "MicroDVD",
        };
        write!(f, "{name}")
    }
}

impl FromStr for SubtitleFormat {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "srt" | "subrip" => Ok(Self::Srt),
            "vtt" | "webvtt" => Ok(Self::WebVtt),
            "ass" | "ssa" => Ok(Self::AdvancedSsa),
            "subviewer" | "sub" => Ok(Self::SubViewer),
            "microdvd" => Ok(Self::MicroDVD),
            _ => Err(Error::format_error(format!("Unsupported subtitle format: {}", s))),
        }
    }
}

/// Represents a time position in a subtitle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    ///
    /// # Arguments
    ///
    /// * `hours` - Hours (0-23)
    /// * `minutes` - Minutes (0-59)
    /// * `seconds` - Seconds (0-59)
    /// * `milliseconds` - Milliseconds (0-999)
    ///
    /// # Returns
    ///
    /// A new TimePosition
    ///
    /// # Panics
    ///
    /// Panics if the time components are out of range
    #[must_use]
    pub fn new(hours: u32, minutes: u32, seconds: u32, milliseconds: u32) -> Self {
        assert!(hours < 24, "Hours must be 0-23");
        assert!(minutes < 60, "Minutes must be 0-59");
        assert!(seconds < 60, "Seconds must be 0-59");
        assert!(milliseconds < 1000, "Milliseconds must be 0-999");
        
        Self {
            hours,
            minutes,
            seconds,
            milliseconds,
        }
    }
    
    /// Creates a time position from a total number of seconds.
    ///
    /// # Arguments
    ///
    /// * `total_seconds` - Total seconds
    ///
    /// # Returns
    ///
    /// A new TimePosition
    #[must_use]
    pub fn from_seconds(total_seconds: f64) -> Self {
        let total_millis = (total_seconds * 1000.0).round() as u32;
        let milliseconds = total_millis % 1000;
        let total_seconds = total_millis / 1000;
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let hours = total_minutes / 60;
        
        Self {
            hours,
            minutes,
            seconds,
            milliseconds,
        }
    }
    
    /// Converts the time position to a total number of seconds.
    ///
    /// # Returns
    ///
    /// Total seconds as a floating-point number
    #[must_use]
    pub fn to_seconds(&self) -> f64 {
        let secs = self.seconds as f64 + (self.milliseconds as f64 / 1000.0);
        let mins = self.minutes as f64 * 60.0;
        let hrs = self.hours as f64 * 3600.0;
        
        hrs + mins + secs
    }
    
    /// Formats the time position in SRT format (00:00:00,000).
    ///
    /// # Returns
    ///
    /// Time position formatted as a string
    #[must_use]
    pub fn to_srt_format(&self) -> String {
        format!(
            "{:02}:{:02}:{:02},{:03}",
            self.hours, self.minutes, self.seconds, self.milliseconds
        )
    }
    
    /// Formats the time position in VTT format (00:00:00.000).
    ///
    /// # Returns
    ///
    /// Time position formatted as a string
    #[must_use]
    pub fn to_vtt_format(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            self.hours, self.minutes, self.seconds, self.milliseconds
        )
    }
    
    /// Parses a time position from SRT format (00:00:00,000).
    ///
    /// # Arguments
    ///
    /// * `time_str` - Time string in SRT format
    ///
    /// # Returns
    ///
    /// Parsed TimePosition or an error
    ///
    /// # Errors
    ///
    /// Returns an error if the time string is invalid
    pub fn parse_srt(time_str: &str) -> Result<Self> {
        Self::parse_time(time_str, ',')
    }
    
    /// Parses a time position from VTT format (00:00:00.000).
    ///
    /// # Arguments
    ///
    /// * `time_str` - Time string in VTT format
    ///
    /// # Returns
    ///
    /// Parsed TimePosition or an error
    ///
    /// # Errors
    ///
    /// Returns an error if the time string is invalid
    pub fn parse_vtt(time_str: &str) -> Result<Self> {
        Self::parse_time(time_str, '.')
    }
    
    /// Parses a time position from a string with the specified separator.
    ///
    /// # Arguments
    ///
    /// * `time_str` - Time string to parse
    /// * `separator` - Separator between seconds and milliseconds
    ///
    /// # Returns
    ///
    /// Parsed TimePosition or an error
    ///
    /// # Errors
    ///
    /// Returns an error if the time string is invalid
    fn parse_time(time_str: &str, separator: char) -> Result<Self> {
        // Expected format: 00:00:00{separator}000
        let parts: Vec<&str> = time_str.split(separator).collect();
        if parts.len() != 2 {
            return Err(Error::timing_error(
                format!("Invalid time format: {time_str}")
            ));
        }
        
        let time_parts: Vec<&str> = parts[0].split(':').collect();
        if time_parts.len() != 3 {
            return Err(Error::timing_error(
                format!("Invalid time format: {time_str}")
            ));
        }
        
        let parse_component = |s: &str, name: &str| -> Result<u32> {
            s.parse::<u32>().map_err(|_| 
                Error::timing_error(format!("Invalid {name} in time: {s}"))
            )
        };
        
        let hours = parse_component(time_parts[0], "hours")?;
        let minutes = parse_component(time_parts[1], "minutes")?;
        let seconds = parse_component(time_parts[2], "seconds")?;
        let milliseconds = parse_component(parts[1], "milliseconds")?;
        
        // Validate ranges
        if hours >= 24 {
            return Err(Error::timing_error(
                format!("Hours out of range (0-23): {hours}")
            ));
        }
        if minutes >= 60 {
            return Err(Error::timing_error(
                format!("Minutes out of range (0-59): {minutes}")
            ));
        }
        if seconds >= 60 {
            return Err(Error::timing_error(
                format!("Seconds out of range (0-59): {seconds}")
            ));
        }
        if milliseconds >= 1000 {
            return Err(Error::timing_error(
                format!("Milliseconds out of range (0-999): {milliseconds}")
            ));
        }
        
        Ok(Self {
            hours,
            minutes,
            seconds,
            milliseconds,
        })
    }

    pub fn as_seconds(&self) -> f64 {
        self.to_seconds()
    }

    pub fn to_srt_string(&self) -> String {
        format!(
            "{:02}:{:02}:{:02},{:03}",
            self.hours, self.minutes, self.seconds, self.milliseconds
        )
    }

    pub fn to_vtt_string(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            self.hours, self.minutes, self.seconds, self.milliseconds
        )
    }

    pub fn from_srt_string(s: &str) -> Result<Self> {
        Self::parse_srt(s)
    }

    pub fn from_vtt_string(s: &str) -> Result<Self> {
        Self::parse_vtt(s)
    }
}

impl FromStr for TimePosition {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self> {
        // Try SRT format first
        if s.contains(',') {
            return Self::parse_srt(s);
        }
        
        // Then try VTT format
        if s.contains('.') {
            return Self::parse_vtt(s);
        }
        
        Err(Error::timing_error(
            format!("Unrecognized time format: {s}")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_subtitle_format_detection_from_extension() {
        assert_eq!(SubtitleFormat::from_extension("test.srt").unwrap(), SubtitleFormat::Srt);
        assert_eq!(SubtitleFormat::from_extension("test.vtt").unwrap(), SubtitleFormat::Vtt);
        assert_eq!(SubtitleFormat::from_extension("test.ass").unwrap(), SubtitleFormat::Ass);
        assert_eq!(SubtitleFormat::from_extension("test.ssa").unwrap(), SubtitleFormat::Ass);
        assert_eq!(SubtitleFormat::from_extension("test.sub").unwrap(), SubtitleFormat::SubViewer);
        
        assert!(SubtitleFormat::from_extension("test.txt").is_err());
    }
    
    #[test]
    fn test_time_position_conversions() {
        let time = TimePosition::new(1, 23, 45, 678);
        assert_eq!(time.to_srt_format(), "01:23:45,678");
        assert_eq!(time.to_vtt_format(), "01:23:45.678");
        
        let seconds = time.to_seconds();
        assert!((seconds - 5025.678).abs() < 0.001);
        
        let time2 = TimePosition::from_seconds(seconds);
        assert_eq!(time, time2);
    }
    
    #[test]
    fn test_time_position_parsing() {
        let srt_time = "01:23:45,678";
        let vtt_time = "01:23:45.678";
        
        let time1 = TimePosition::parse_srt(srt_time).unwrap();
        let time2 = TimePosition::parse_vtt(vtt_time).unwrap();
        
        assert_eq!(time1, time2);
        assert_eq!(time1.hours, 1);
        assert_eq!(time1.minutes, 23);
        assert_eq!(time1.seconds, 45);
        assert_eq!(time1.milliseconds, 678);
        
        assert!(TimePosition::parse_srt("invalid").is_err());
        assert!(TimePosition::parse_vtt("01:23:invalid").is_err());
    }
    
    #[test]
    fn test_format_display_and_parsing() {
        let formats = [
            SubtitleFormat::Srt,
            SubtitleFormat::Vtt,
            SubtitleFormat::WebVtt,
            SubtitleFormat::Ass,
            SubtitleFormat::AdvancedSsa,
            SubtitleFormat::SubViewer,
            SubtitleFormat::MicroDVD,
        ];
        
        for format in formats {
            let s = format.to_string();
            let parsed: SubtitleFormat = s.parse().unwrap();
            assert_eq!(format, parsed);
        }
        
        assert!("Unknown".parse::<SubtitleFormat>().is_err());
    }
} 