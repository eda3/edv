/// Subtitle data model.
///
/// This module defines the core data structures for representing
/// subtitles, including text content, timing, and styling.
use std::collections::HashMap;
use std::str::FromStr;

use crate::subtitle::error::{Error, Result};
use crate::subtitle::format::{SubtitleFormat, TimePosition};
use crate::subtitle::style::TextStyle;

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

impl Subtitle {
    /// Creates a new subtitle.
    ///
    /// # Arguments
    ///
    /// * `start_time` - Start time for displaying the subtitle
    /// * `end_time` - End time for displaying the subtitle
    /// * `text` - Text content of the subtitle
    ///
    /// # Returns
    ///
    /// A new Subtitle instance
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

    /// Sets the subtitle ID.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the subtitle style.
    #[must_use]
    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Gets the ID of the subtitle.
    #[must_use]
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Sets the ID of the subtitle.
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = id.into();
    }

    /// Gets the start time of the subtitle.
    #[must_use]
    pub fn get_start(&self) -> TimePosition {
        self.start_time
    }

    /// Sets the start time of the subtitle.
    pub fn set_start(&mut self, time: TimePosition) {
        self.start_time = time;
    }

    /// Gets the end time of the subtitle.
    #[must_use]
    pub fn get_end(&self) -> TimePosition {
        self.end_time
    }

    /// Sets the end time of the subtitle.
    pub fn set_end(&mut self, time: TimePosition) {
        self.end_time = time;
    }

    /// Gets the text content of the subtitle.
    #[must_use]
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Sets the text content of the subtitle.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Gets the style of the subtitle.
    #[must_use]
    pub fn get_style(&self) -> &Option<TextStyle> {
        &self.style
    }

    /// Sets the style of the subtitle.
    pub fn set_style(&mut self, style: TextStyle) {
        self.style = Some(style);
    }

    /// Gets the duration of the subtitle in seconds.
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.end_time.as_seconds() - self.start_time.as_seconds()
    }

    /// Checks if this subtitle overlaps with another subtitle.
    ///
    /// # Arguments
    ///
    /// * `other` - Another subtitle to check for overlap
    ///
    /// # Returns
    ///
    /// `true` if the subtitles overlap in time, `false` otherwise
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
    ///
    /// # Arguments
    ///
    /// * `seconds` - Number of seconds to shift (positive = later, negative = earlier)
    pub fn shift(&mut self, seconds: f64) {
        let start_seconds = self.start_time.as_seconds() + seconds;
        let end_seconds = self.end_time.as_seconds() + seconds;

        // Ensure times don't go below zero
        let start_seconds = start_seconds.max(0.0);
        let end_seconds = end_seconds.max(start_seconds + 0.1); // Ensure minimum duration

        self.start_time = TimePosition::from_seconds(start_seconds);
        self.end_time = TimePosition::from_seconds(end_seconds);
    }

    /// Adjusts the subtitle timing by a scaling factor.
    ///
    /// # Arguments
    ///
    /// * `factor` - Scaling factor (e.g., 1.05 = 5% slower, 0.95 = 5% faster)
    /// * `offset` - Time offset (in seconds) to apply the scaling from
    pub fn scale_timing(&mut self, factor: f64, offset: f64) {
        if factor <= 0.0 {
            return; // Invalid factor, return unchanged
        }

        let start_seconds = self.start_time.as_seconds();
        let end_seconds = self.end_time.as_seconds();

        let new_start = offset + (start_seconds - offset) * factor;
        let new_end = offset + (end_seconds - offset) * factor;

        let new_start = new_start.max(0.0);
        let new_end = new_end.max(new_start + 0.1); // Ensure minimum duration

        self.start_time = TimePosition::from_seconds(new_start);
        self.end_time = TimePosition::from_seconds(new_end);
    }

    /// Formats the subtitle in SRT format.
    ///
    /// # Returns
    ///
    /// The subtitle formatted as an SRT string
    #[must_use]
    pub fn to_srt(&self) -> String {
        format!(
            "{}\n{} --> {}\n{}\n",
            self.id,
            self.start_time.to_srt_string(),
            self.end_time.to_srt_string(),
            self.text
        )
    }

    /// Formats the subtitle in WebVTT format.
    ///
    /// # Returns
    ///
    /// The subtitle formatted as a WebVTT string
    #[must_use]
    pub fn to_vtt(&self) -> String {
        let mut result = String::new();

        // Add cue identifier if present and not empty
        if !self.id.is_empty() {
            result.push_str(&self.id);
            result.push('\n');
        }

        // Add timing line
        result.push_str(&format!(
            "{} --> {}",
            self.start_time.to_vtt_string(),
            self.end_time.to_vtt_string()
        ));

        // Add style settings if present
        if let Some(style) = &self.style {
            let settings = style.to_vtt_string();
            if !settings.is_empty() {
                result.push(' ');
                result.push_str(&settings);
            }
        }

        // Add text content
        result.push('\n');
        result.push_str(&self.text);
        result.push('\n');

        result
    }
}

/// A collection of subtitles.
#[derive(Debug, Clone, Default)]
pub struct SubtitleTrack {
    /// Collection of subtitles by ID
    subtitles: HashMap<String, Subtitle>,
    /// Ordered list of subtitle IDs for iteration
    order: Vec<String>,
}

impl SubtitleTrack {
    /// Creates a new empty subtitle track.
    #[must_use]
    pub fn new() -> Self {
        Self {
            subtitles: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Returns the number of subtitles in the track.
    #[must_use]
    pub fn len(&self) -> usize {
        self.subtitles.len()
    }

    /// Checks if the track is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subtitles.is_empty()
    }

    /// Adds a subtitle to the track.
    pub fn add_subtitle(&mut self, subtitle: Subtitle) {
        let id = subtitle.get_id().to_string();

        // If ID is empty, generate a new one
        let id = if id.is_empty() {
            let new_id = (self.subtitles.len() + 1).to_string();
            let mut sub = subtitle;
            sub.set_id(new_id.clone());
            self.subtitles.insert(new_id.clone(), sub);
            new_id
        } else {
            self.subtitles.insert(id.clone(), subtitle);
            id
        };

        // Add to order if not already there
        if !self.order.contains(&id) {
            self.order.push(id);
        }
    }

    /// Removes a subtitle by ID.
    ///
    /// # Returns
    ///
    /// `true` if the subtitle was removed, `false` if not found
    pub fn remove_subtitle(&mut self, id: &str) -> bool {
        if self.subtitles.remove(id).is_some() {
            self.order.retain(|i| i != id);
            true
        } else {
            false
        }
    }

    /// Replaces a subtitle with the same ID.
    pub fn replace_subtitle(&mut self, subtitle: Subtitle) {
        let id = subtitle.get_id().to_string();
        if !id.is_empty() {
            self.subtitles.insert(id.clone(), subtitle);

            // Add to order if not already there
            if !self.order.contains(&id) {
                self.order.push(id);
            }
        }
    }

    /// Gets a reference to a subtitle by ID.
    #[must_use]
    pub fn get_subtitle(&self, id: &str) -> Option<&Subtitle> {
        self.subtitles.get(id)
    }

    /// Gets a mutable reference to a subtitle by ID.
    pub fn get_subtitle_mut(&mut self, id: &str) -> Option<&mut Subtitle> {
        self.subtitles.get_mut(id)
    }

    /// Gets all subtitles as a slice in order.
    #[must_use]
    pub fn get_subtitles(&self) -> Vec<&Subtitle> {
        self.order
            .iter()
            .filter_map(|id| self.subtitles.get(id))
            .collect()
    }

    /// Gets mutable references to all subtitles.
    ///
    /// # Returns
    ///
    /// A vector of mutable references to subtitles
    pub fn get_subtitles_mut(&mut self) -> Vec<&mut Subtitle> {
        // 安全でない方法を避けるため、異なるアプローチが必要
        // Vec<&mut>を返すことは基本的に難しい場合がある

        // 代わりに、HashMap内の値を直接取得するのではなく
        // Vecの構築にはHashMap::values_mutを使用する
        self.subtitles.values_mut().collect()
    }

    /// Gets subtitles in a specific time range.
    ///
    /// # Arguments
    ///
    /// * `start` - Start time in seconds
    /// * `end` - End time in seconds
    ///
    /// # Returns
    ///
    /// A vector of references to subtitles that overlap with the specified time range
    #[must_use]
    pub fn get_subtitles_in_range(&self, start: f64, end: f64) -> Vec<&Subtitle> {
        let start_time = TimePosition::from_seconds(start);
        let end_time = TimePosition::from_seconds(end);

        self.get_subtitles()
            .into_iter()
            .filter(|s| s.get_end().as_seconds() > start && s.get_start().as_seconds() < end)
            .collect()
    }

    /// Sorts all subtitles by their start time.
    pub fn sort(&mut self) {
        // Sort the order vector based on subtitle start times
        self.order.sort_by(|a, b| {
            let a_time = self
                .subtitles
                .get(a)
                .map_or(0.0, |s| s.get_start().as_seconds());
            let b_time = self
                .subtitles
                .get(b)
                .map_or(0.0, |s| s.get_start().as_seconds());
            a_time
                .partial_cmp(&b_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Shifts all subtitles by the specified number of seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Number of seconds to shift (positive = later, negative = earlier)
    pub fn shift_all(&mut self, seconds: f64) {
        for subtitle in self.get_subtitles_mut() {
            subtitle.shift(seconds);
        }
    }

    /// Finds overlapping subtitles in the track.
    ///
    /// # Returns
    ///
    /// A vector of pairs of overlapping subtitle IDs
    #[must_use]
    pub fn find_overlaps(&self) -> Vec<(String, String)> {
        let mut overlaps = Vec::new();
        let subtitles = self.get_subtitles();

        for (i, sub1) in subtitles.iter().enumerate() {
            for sub2 in subtitles.iter().skip(i + 1) {
                if sub1.overlaps_with(sub2) {
                    overlaps.push(((*sub1).get_id().to_string(), (*sub2).get_id().to_string()));
                }
            }
        }

        overlaps
    }

    /// Formats the entire subtitle track as SRT.
    ///
    /// # Returns
    ///
    /// The formatted SRT string
    #[must_use]
    pub fn format_as_srt(&self) -> String {
        let mut result = String::new();
        let mut subtitles = self.get_subtitles();

        // Sort subtitles by start time
        subtitles.sort_by(|a, b| {
            a.get_start()
                .as_seconds()
                .partial_cmp(&b.get_start().as_seconds())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Format each subtitle with correct numbering
        for (i, subtitle) in subtitles.iter().enumerate() {
            let mut sub_copy = (*subtitle).clone();
            sub_copy.set_id((i + 1).to_string());
            result.push_str(&sub_copy.to_srt());
            result.push('\n');
        }

        result
    }

    /// Formats the entire subtitle track as WebVTT.
    ///
    /// # Returns
    ///
    /// The formatted WebVTT string
    #[must_use]
    pub fn format_as_vtt(&self) -> String {
        let mut result = String::from("WEBVTT\n\n");
        let mut subtitles = self.get_subtitles();

        // Sort subtitles by start time
        subtitles.sort_by(|a, b| {
            a.get_start()
                .as_seconds()
                .partial_cmp(&b.get_start().as_seconds())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Format each subtitle
        for subtitle in subtitles {
            result.push_str(&subtitle.to_vtt());
            result.push('\n');
        }

        result
    }

    /// Gets all subtitle IDs in this track.
    ///
    /// # Returns
    ///
    /// A vector containing all subtitle IDs.
    pub fn get_subtitle_ids(&self) -> Vec<String> {
        self.subtitles.keys().cloned().collect()
    }
}

/// Builder for creating subtitles.
#[derive(Debug, Default)]
pub struct SubtitleBuilder {
    /// ID of the subtitle
    id: Option<String>,

    /// Start time of the subtitle
    start_time: Option<TimePosition>,

    /// End time of the subtitle
    end_time: Option<TimePosition>,

    /// Duration of the subtitle in seconds (alternative to end_time)
    duration: Option<f64>,

    /// Text content of the subtitle
    text: String,

    /// Style information for the subtitle
    style: Option<TextStyle>,
}

impl SubtitleBuilder {
    /// Creates a new subtitle builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the subtitle ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the start time.
    #[must_use]
    pub fn start(mut self, time: TimePosition) -> Self {
        self.start_time = Some(time);
        self
    }

    /// Sets the start time in seconds.
    #[must_use]
    pub fn start_seconds(mut self, seconds: f64) -> Self {
        self.start_time = Some(TimePosition::from_seconds(seconds));
        self
    }

    /// Sets the end time.
    #[must_use]
    pub fn end(mut self, time: TimePosition) -> Self {
        self.end_time = Some(time);
        self
    }

    /// Sets the end time in seconds.
    #[must_use]
    pub fn end_seconds(mut self, seconds: f64) -> Self {
        self.end_time = Some(TimePosition::from_seconds(seconds));
        self
    }

    /// Sets the duration in seconds (alternative to setting end time).
    #[must_use]
    pub fn duration(mut self, seconds: f64) -> Self {
        self.duration = Some(seconds);
        self
    }

    /// Sets the text content.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the style information.
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Builds the subtitle.
    ///
    /// # Returns
    ///
    /// A Result containing either the built Subtitle or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Start time is not set
    /// * Neither end time nor duration is set
    /// * End time is before start time
    pub fn build(self) -> Result<Subtitle> {
        // Check required fields
        let start_time = self
            .start_time
            .ok_or_else(|| Error::timing_error("Start time must be set".to_string()))?;

        let end_time = if let Some(end) = self.end_time {
            end
        } else if let Some(duration) = self.duration {
            if duration <= 0.0 {
                return Err(Error::timing_error("Duration must be positive".to_string()));
            }
            TimePosition::from_seconds(start_time.as_seconds() + duration)
        } else {
            return Err(Error::timing_error(
                "Either end time or duration must be set".to_string(),
            ));
        };

        // Validate timing
        if start_time.as_seconds() >= end_time.as_seconds() {
            return Err(Error::timing_error(
                "End time must be after start time".to_string(),
            ));
        }

        // Create the subtitle
        let mut subtitle = Subtitle {
            id: self.id.unwrap_or_else(String::new),
            start_time,
            end_time,
            text: self.text,
            style: self.style,
        };

        Ok(subtitle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_creation() {
        let start = TimePosition::from_seconds(1.0);
        let end = TimePosition::from_seconds(4.0);
        let subtitle = Subtitle::new(start, end, "Hello, world!");

        assert_eq!(subtitle.get_start().as_seconds(), 1.0);
        assert_eq!(subtitle.get_end().as_seconds(), 4.0);
        assert_eq!(subtitle.get_text(), "Hello, world!");
        assert_eq!(subtitle.duration(), 3.0);
    }

    #[test]
    fn test_subtitle_overlap() {
        let sub1 = Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(4.0),
            "First",
        );

        let sub2 = Subtitle::new(
            TimePosition::from_seconds(3.0),
            TimePosition::from_seconds(6.0),
            "Second",
        );

        let sub3 = Subtitle::new(
            TimePosition::from_seconds(5.0),
            TimePosition::from_seconds(8.0),
            "Third",
        );

        assert!(sub1.overlaps_with(&sub2));
        assert!(sub2.overlaps_with(&sub3));
        assert!(!sub1.overlaps_with(&sub3));
    }

    #[test]
    fn test_subtitle_shift() {
        let mut subtitle = Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(4.0),
            "Test",
        );

        subtitle.shift(2.0);
        assert_eq!(subtitle.get_start().as_seconds(), 3.0);
        assert_eq!(subtitle.get_end().as_seconds(), 6.0);

        // Test negative shift
        subtitle.shift(-1.0);
        assert_eq!(subtitle.get_start().as_seconds(), 2.0);
        assert_eq!(subtitle.get_end().as_seconds(), 5.0);
    }

    #[test]
    fn test_subtitle_track() {
        let mut track = SubtitleTrack::new();
        assert!(track.is_empty());

        // Add subtitles
        track.add_subtitle(
            Subtitle::new(
                TimePosition::from_seconds(1.0),
                TimePosition::from_seconds(4.0),
                "First",
            )
            .with_id("1"),
        );

        track.add_subtitle(
            Subtitle::new(
                TimePosition::from_seconds(5.0),
                TimePosition::from_seconds(8.0),
                "Second",
            )
            .with_id("2"),
        );

        assert_eq!(track.len(), 2);

        // Get subtitle by ID
        let sub = track.get_subtitle("1").unwrap();
        assert_eq!(sub.get_text(), "First");

        // Get subtitles in range
        let in_range = track.get_subtitles_in_range(3.0, 6.0);
        assert_eq!(in_range.len(), 2);

        // Remove subtitle
        assert!(track.remove_subtitle("1"));
        assert_eq!(track.len(), 1);
    }

    #[test]
    fn test_subtitle_builder() {
        let subtitle = SubtitleBuilder::new()
            .start_seconds(1.0)
            .duration(3.0)
            .text("Builder test")
            .id("test")
            .build()
            .unwrap();

        assert_eq!(subtitle.get_id(), "test");
        assert_eq!(subtitle.get_start().as_seconds(), 1.0);
        assert_eq!(subtitle.get_end().as_seconds(), 4.0);
        assert_eq!(subtitle.get_text(), "Builder test");

        // Test validation
        let result = SubtitleBuilder::new()
            .start_seconds(5.0)
            .end_seconds(3.0) // End before start, should fail
            .text("Invalid")
            .build();

        assert!(result.is_err());
    }
}
