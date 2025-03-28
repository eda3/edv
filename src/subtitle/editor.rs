/// Subtitle editing functionality.
///
/// This module provides tools for editing subtitle tracks,
/// including adding, removing, and modifying subtitles,
/// as well as advanced operations like merging, splitting,
/// and batch processing.

use std::path::Path;

use crate::subtitle::error::{Error, Result};
use crate::subtitle::format::{SubtitleFormat, TimePosition};
use crate::subtitle::model::{Subtitle, SubtitleTrack};
use crate::subtitle::parser;
use crate::subtitle::style::TextStyle;

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

impl ShiftBuilder {
    /// Creates a new shift operation with the specified amount in seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Time to shift in seconds (positive or negative)
    #[must_use]
    pub fn new(seconds: f64) -> Self {
        Self {
            shift_seconds: seconds,
            after: None,
            before: None,
            ids: Vec::new(),
        }
    }
    
    /// Limits the shift operation to subtitles that occur after the specified time.
    ///
    /// # Arguments
    ///
    /// * `position` - Only shift subtitles after this time position
    #[must_use]
    pub fn after(mut self, position: TimePosition) -> Self {
        self.after = Some(position);
        self
    }
    
    /// Limits the shift operation to subtitles that occur before the specified time.
    ///
    /// # Arguments
    ///
    /// * `position` - Only shift subtitles before this time position
    #[must_use]
    pub fn before(mut self, position: TimePosition) -> Self {
        self.before = Some(position);
        self
    }
    
    /// Limits the shift operation to subtitles with the specified IDs.
    ///
    /// # Arguments
    ///
    /// * `subtitle_ids` - List of subtitle IDs to shift
    #[must_use]
    pub fn with_ids(mut self, subtitle_ids: Vec<String>) -> Self {
        self.ids = subtitle_ids;
        self
    }
    
    /// Applies the shift operation to a subtitle track.
    ///
    /// # Arguments
    ///
    /// * `track` - The subtitle track to modify
    ///
    /// # Returns
    ///
    /// The number of subtitles that were shifted
    pub fn apply(&self, track: &mut SubtitleTrack) -> usize {
        let mut count = 0;
        
        for subtitle in track.get_subtitles_mut() {
            // Check if this subtitle should be shifted
            let should_shift = self.should_shift_subtitle(subtitle);
            
            if should_shift {
                subtitle.shift(self.shift_seconds);
                count += 1;
            }
        }
        
        count
    }
    
    // Helper method to determine if a subtitle should be shifted
    fn should_shift_subtitle(&self, subtitle: &Subtitle) -> bool {
        // Check ID filter
        if !self.ids.is_empty() && !self.ids.contains(&subtitle.get_id()) {
            return false;
        }
        
        // Check time range filters
        if let Some(after) = self.after {
            if subtitle.get_start() < after {
                return false;
            }
        }
        
        if let Some(before) = self.before {
            if subtitle.get_start() > before {
                return false;
            }
        }
        
        true
    }
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
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the subtitle file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_ref = path.as_ref();
        
        // Parse the file
        let track = parser::parse_subtitle_file(path_ref)?;
        
        // Determine format from extension
        let format = if let Some(ext) = path_ref.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "srt" => SubtitleFormat::Srt,
                "vtt" => SubtitleFormat::WebVtt,
                "ass" | "ssa" => SubtitleFormat::AdvancedSsa,
                _ => SubtitleFormat::Srt, // Default to SRT
            }
        } else {
            SubtitleFormat::Srt
        };
        
        // Update the editor state
        self.track = track;
        self.format = format;
        self.file_path = path_ref.to_str().map(String::from);
        self.modified = false;
        
        Ok(())
    }
    
    /// Creates a new subtitle track from scratch.
    ///
    /// # Arguments
    ///
    /// * `format` - The subtitle format to use
    pub fn create_new(&mut self, format: SubtitleFormat) {
        self.track = SubtitleTrack::new();
        self.format = format;
        self.file_path = None;
        self.modified = true;
    }
    
    /// Adds a new subtitle to the track.
    ///
    /// # Arguments
    ///
    /// * `subtitle` - The subtitle to add
    pub fn add_subtitle(&mut self, subtitle: Subtitle) {
        self.track.add_subtitle(subtitle);
        self.modified = true;
    }
    
    /// Removes a subtitle by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to remove
    ///
    /// # Returns
    ///
    /// `true` if a subtitle was removed, `false` otherwise
    pub fn remove_subtitle(&mut self, id: &str) -> bool {
        let result = self.track.remove_subtitle(id);
        if result {
            self.modified = true;
        }
        result
    }
    
    /// Gets a reference to a subtitle by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to get
    ///
    /// # Returns
    ///
    /// An optional reference to the subtitle
    #[must_use]
    pub fn get_subtitle(&self, id: &str) -> Option<&Subtitle> {
        self.track.get_subtitle(id)
    }
    
    /// Gets a mutable reference to a subtitle by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to get
    ///
    /// # Returns
    ///
    /// An optional mutable reference to the subtitle
    pub fn get_subtitle_mut(&mut self, id: &str) -> Option<&mut Subtitle> {
        self.modified = true;
        self.track.get_subtitle_mut(id)
    }
    
    /// Gets all subtitles in the track.
    #[must_use]
    pub fn get_subtitles(&self) -> &[Subtitle] {
        self.track.get_subtitles()
    }
    
    /// Updates the text of a subtitle.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to update
    /// * `text` - The new text content
    ///
    /// # Returns
    ///
    /// `true` if the subtitle was updated, `false` if not found
    pub fn update_text(&mut self, id: &str, text: &str) -> bool {
        if let Some(subtitle) = self.track.get_subtitle_mut(id) {
            subtitle.set_text(text);
            self.modified = true;
            true
        } else {
            false
        }
    }
    
    /// Updates the style of a subtitle.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to update
    /// * `style` - The new style
    ///
    /// # Returns
    ///
    /// `true` if the subtitle was updated, `false` if not found
    pub fn update_style(&mut self, id: &str, style: TextStyle) -> bool {
        if let Some(subtitle) = self.track.get_subtitle_mut(id) {
            subtitle.set_style(style);
            self.modified = true;
            true
        } else {
            false
        }
    }
    
    /// Updates the timing of a subtitle.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to update
    /// * `start` - The new start time
    /// * `end` - The new end time
    ///
    /// # Returns
    ///
    /// `true` if the subtitle was updated, `false` if not found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The start time is after the end time
    pub fn update_timing(&mut self, id: &str, start: TimePosition, end: TimePosition) -> Result<bool> {
        if start >= end {
            return Err(Error::invalid_time_range(start, end));
        }
        
        if let Some(subtitle) = self.track.get_subtitle_mut(id) {
            subtitle.set_start(start);
            subtitle.set_end(end);
            self.modified = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Shifts the timing of all or selected subtitles.
    ///
    /// # Arguments
    ///
    /// * `shift` - The shift operation to apply
    ///
    /// # Returns
    ///
    /// The number of subtitles that were shifted
    pub fn shift_subtitles(&mut self, shift: ShiftBuilder) -> usize {
        let count = shift.apply(&mut self.track);
        if count > 0 {
            self.modified = true;
        }
        count
    }
    
    /// Merges adjacent subtitles that meet specific criteria.
    ///
    /// # Arguments
    ///
    /// * `max_gap` - Maximum time gap between subtitles to merge (in seconds)
    /// * `same_style` - If true, only merge subtitles with the same style
    ///
    /// # Returns
    ///
    /// The number of merges performed
    pub fn merge_adjacent_subtitles(&mut self, max_gap: f64, same_style: bool) -> usize {
        let mut merged_count = 0;
        let mut i = 0;
        
        while i < self.track.len().saturating_sub(1) {
            let current = &self.track.get_subtitles()[i].clone();
            let next = &self.track.get_subtitles()[i + 1].clone();
            
            let gap = next.get_start().as_seconds() - current.get_end().as_seconds();
            
            let can_merge = gap <= max_gap &&
                (!same_style || current.get_style() == next.get_style());
            
            if can_merge {
                // Create a new merged subtitle
                let merged_text = format!("{}\n{}", current.get_text(), next.get_text());
                let merged = Subtitle::new(
                    current.get_start(),
                    next.get_end(),
                    merged_text
                )
                .with_id(current.get_id());
                
                if let Some(style) = current.get_style().clone() {
                    self.track.replace_subtitle(merged.with_style(style));
                } else {
                    self.track.replace_subtitle(merged);
                }
                
                // Remove the second subtitle
                self.track.remove_subtitle(&next.get_id());
                merged_count += 1;
            } else {
                i += 1;
            }
        }
        
        if merged_count > 0 {
            self.modified = true;
        }
        
        merged_count
    }
    
    /// Splits a subtitle at the specified time.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the subtitle to split
    /// * `split_time` - The time position to split at
    ///
    /// # Returns
    ///
    /// `true` if the subtitle was split, `false` if not found or can't be split
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The split time is outside the subtitle's duration
    pub fn split_subtitle(&mut self, id: &str, split_time: TimePosition) -> Result<bool> {
        let subtitle = match self.track.get_subtitle(id) {
            Some(s) => s.clone(),
            None => return Ok(false),
        };
        
        let start = subtitle.get_start();
        let end = subtitle.get_end();
        
        if split_time <= start || split_time >= end {
            return Err(Error::invalid_split_point(split_time, start, end));
        }
        
        // Create two new subtitles
        let first_half = Subtitle::new(
            start,
            split_time,
            subtitle.get_text()
        )
        .with_id(id.to_string());
        
        // Generate a new ID for the second subtitle
        let new_id = format!("{}-1", id);
        let second_half = Subtitle::new(
            split_time,
            end,
            subtitle.get_text()
        )
        .with_id(new_id);
        
        // Apply styles if present
        if let Some(style) = subtitle.get_style().clone() {
            self.track.replace_subtitle(first_half.with_style(style.clone()));
            self.track.add_subtitle(second_half.with_style(style));
        } else {
            self.track.replace_subtitle(first_half);
            self.track.add_subtitle(second_half);
        }
        
        self.modified = true;
        Ok(true)
    }
    
    /// Sorts all subtitles by their start time.
    pub fn sort_subtitles(&mut self) {
        self.track.sort();
    }
    
    /// Checks for and fixes overlapping subtitles.
    ///
    /// # Arguments
    ///
    /// * `strategy` - The strategy to use for fixing overlaps:
    ///   - "shorten": Shorten the first subtitle to end when the next one starts
    ///   - "gap": Ensure there's a small gap between subtitles
    ///
    /// # Returns
    ///
    /// The number of overlaps fixed
    pub fn fix_overlaps(&mut self, strategy: &str, min_gap: f64) -> usize {
        self.sort_subtitles();
        
        let mut fixed_count = 0;
        let subtitles = self.track.get_subtitles().to_vec();
        
        for i in 0..subtitles.len().saturating_sub(1) {
            let first = &subtitles[i];
            let second = &subtitles[i + 1];
            
            if first.get_end() > second.get_start() {
                // We have an overlap
                if strategy == "shorten" {
                    // Shorten the first subtitle
                    if let Some(subtitle) = self.track.get_subtitle_mut(&first.get_id()) {
                        subtitle.set_end(second.get_start());
                        fixed_count += 1;
                    }
                } else if strategy == "gap" {
                    // Create a gap by adjusting both subtitles
                    let mid_point = (first.get_end().as_seconds() + 
                                     second.get_start().as_seconds()) / 2.0;
                    
                    let half_gap = min_gap / 2.0;
                    let first_end = TimePosition::from_seconds(mid_point - half_gap);
                    let second_start = TimePosition::from_seconds(mid_point + half_gap);
                    
                    if let Some(subtitle) = self.track.get_subtitle_mut(&first.get_id()) {
                        subtitle.set_end(first_end);
                    }
                    
                    if let Some(subtitle) = self.track.get_subtitle_mut(&second.get_id()) {
                        subtitle.set_start(second_start);
                    }
                    
                    fixed_count += 1;
                }
            }
        }
        
        if fixed_count > 0 {
            self.modified = true;
        }
        
        fixed_count
    }
    
    /// Saves the current subtitle track to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path to save to. If None, uses the original path if available.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * No path is provided and no original path is available
    /// * The file cannot be written
    pub fn save(&mut self, path: Option<&str>) -> Result<()> {
        let file_path = match path {
            Some(p) => p.to_string(),
            None => match &self.file_path {
                Some(p) => p.clone(),
                None => return Err(Error::no_file_path()),
            },
        };
        
        let content = match self.format {
            SubtitleFormat::Srt => self.track.format_as_srt(),
            SubtitleFormat::WebVtt => self.track.format_as_vtt(),
            _ => return Err(Error::unsupported_export_format(&self.format.to_string())),
        };
        
        std::fs::write(&file_path, content)?;
        
        self.file_path = Some(file_path);
        self.modified = false;
        
        Ok(())
    }
    
    /// Gets the current subtitle format.
    #[must_use]
    pub fn get_format(&self) -> SubtitleFormat {
        self.format
    }
    
    /// Sets the subtitle format for saving.
    pub fn set_format(&mut self, format: SubtitleFormat) {
        self.format = format;
    }
    
    /// Gets the subtitle track being edited.
    #[must_use]
    pub fn get_track(&self) -> &SubtitleTrack {
        &self.track
    }
    
    /// Gets a mutable reference to the subtitle track being edited.
    pub fn get_track_mut(&mut self) -> &mut SubtitleTrack {
        self.modified = true;
        &mut self.track
    }
    
    /// Checks if the subtitle track has been modified since the last save.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.modified
    }
    
    /// Gets the file path if the track was loaded from a file.
    #[must_use]
    pub fn get_file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }
}

impl Default for SubtitleEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    fn create_test_track() -> SubtitleTrack {
        let mut track = SubtitleTrack::new();
        
        // Add some test subtitles
        track.add_subtitle(Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(3.0),
            "First subtitle"
        ).with_id("1"));
        
        track.add_subtitle(Subtitle::new(
            TimePosition::from_seconds(4.0),
            TimePosition::from_seconds(6.0),
            "Second subtitle"
        ).with_id("2"));
        
        track.add_subtitle(Subtitle::new(
            TimePosition::from_seconds(7.0),
            TimePosition::from_seconds(9.0),
            "Third subtitle"
        ).with_id("3"));
        
        track
    }
    
    #[test]
    fn test_editor_add_remove() {
        let mut editor = SubtitleEditor::new();
        assert_eq!(editor.get_track().len(), 0);
        
        // Add a subtitle
        let subtitle = Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(3.0),
            "Test subtitle"
        ).with_id("1");
        
        editor.add_subtitle(subtitle);
        assert_eq!(editor.get_track().len(), 1);
        
        // Remove it
        assert!(editor.remove_subtitle("1"));
        assert_eq!(editor.get_track().len(), 0);
        
        // Try to remove a non-existent subtitle
        assert!(!editor.remove_subtitle("999"));
    }
    
    #[test]
    fn test_update_subtitle() {
        let mut editor = SubtitleEditor::new();
        let track = create_test_track();
        editor.track = track;
        
        // Update text
        assert!(editor.update_text("2", "Updated text"));
        assert_eq!(editor.get_subtitle("2").unwrap().get_text(), "Updated text");
        
        // Update timing
        let start = TimePosition::from_seconds(4.5);
        let end = TimePosition::from_seconds(6.5);
        assert!(editor.update_timing("2", start, end).unwrap());
        
        let subtitle = editor.get_subtitle("2").unwrap();
        assert_eq!(subtitle.get_start().as_seconds(), 4.5);
        assert_eq!(subtitle.get_end().as_seconds(), 6.5);
        
        // Try invalid timing (start after end)
        let result = editor.update_timing(
            "2",
            TimePosition::from_seconds(7.0),
            TimePosition::from_seconds(6.0)
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_shift_subtitles() {
        let mut editor = SubtitleEditor::new();
        let track = create_test_track();
        editor.track = track;
        
        // Shift all subtitles forward by 1 second
        let shift = ShiftBuilder::new(1.0);
        let count = editor.shift_subtitles(shift);
        assert_eq!(count, 3);
        
        // Check that all were shifted
        assert_eq!(editor.get_subtitle("1").unwrap().get_start().as_seconds(), 2.0);
        assert_eq!(editor.get_subtitle("2").unwrap().get_start().as_seconds(), 5.0);
        assert_eq!(editor.get_subtitle("3").unwrap().get_start().as_seconds(), 8.0);
        
        // Shift only subtitles after 5 seconds
        let shift = ShiftBuilder::new(-0.5).after(TimePosition::from_seconds(5.0));
        let count = editor.shift_subtitles(shift);
        assert_eq!(count, 2);
        
        // Check selective shift
        assert_eq!(editor.get_subtitle("1").unwrap().get_start().as_seconds(), 2.0); // unchanged
        assert_eq!(editor.get_subtitle("2").unwrap().get_start().as_seconds(), 4.5); // shifted back
        assert_eq!(editor.get_subtitle("3").unwrap().get_start().as_seconds(), 7.5); // shifted back
    }
    
    #[test]
    fn test_split_subtitle() {
        let mut editor = SubtitleEditor::new();
        
        // Add a subtitle to split
        let subtitle = Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(5.0),
            "This is a subtitle that will be split"
        ).with_id("1");
        
        editor.add_subtitle(subtitle);
        
        // Split it at 3 seconds
        let result = editor.split_subtitle("1", TimePosition::from_seconds(3.0));
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Check we now have two subtitles
        assert_eq!(editor.get_track().len(), 2);
        
        // Check the split results
        let first = editor.get_subtitle("1").unwrap();
        assert_eq!(first.get_start().as_seconds(), 1.0);
        assert_eq!(first.get_end().as_seconds(), 3.0);
        
        let second = editor.get_subtitle("1-1").unwrap();
        assert_eq!(second.get_start().as_seconds(), 3.0);
        assert_eq!(second.get_end().as_seconds(), 5.0);
    }
    
    #[test]
    fn test_fix_overlaps() {
        let mut editor = SubtitleEditor::new();
        
        // Add overlapping subtitles
        editor.add_subtitle(Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(4.0),
            "First subtitle"
        ).with_id("1"));
        
        editor.add_subtitle(Subtitle::new(
            TimePosition::from_seconds(3.0), // Overlaps with the first
            TimePosition::from_seconds(6.0),
            "Second subtitle"
        ).with_id("2"));
        
        // Fix overlaps with 'shorten' strategy
        let fixed = editor.fix_overlaps("shorten", 0.0);
        assert_eq!(fixed, 1);
        
        // Check that the first subtitle was shortened
        let first = editor.get_subtitle("1").unwrap();
        assert_eq!(first.get_end().as_seconds(), 3.0);
        
        // Reset and try with 'gap' strategy
        editor.get_subtitle_mut("1").unwrap().set_end(TimePosition::from_seconds(4.0));
        
        let fixed = editor.fix_overlaps("gap", 0.5);
        assert_eq!(fixed, 1);
        
        // Check that both subtitles were adjusted to create a gap
        let first = editor.get_subtitle("1").unwrap();
        let second = editor.get_subtitle("2").unwrap();
        
        assert!(first.get_end().as_seconds() < second.get_start().as_seconds());
        assert!(second.get_start().as_seconds() - first.get_end().as_seconds() >= 0.5);
    }
    
    #[test]
    fn test_save_and_load() {
        let mut editor = SubtitleEditor::new();
        let track = create_test_track();
        editor.track = track;
        editor.format = SubtitleFormat::Srt;
        
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        
        // Save to the temp file
        editor.save(Some(&path)).unwrap();
        assert!(!editor.is_modified());
        
        // Create a new editor and load the file
        let mut new_editor = SubtitleEditor::new();
        new_editor.load_file(&path).unwrap();
        
        // Check it loaded correctly
        assert_eq!(new_editor.get_track().len(), 3);
        assert!(new_editor.get_subtitle("1").is_some());
        assert!(new_editor.get_subtitle("2").is_some());
        assert!(new_editor.get_subtitle("3").is_some());
    }
    
    #[test]
    fn test_create_new() {
        let mut editor = SubtitleEditor::new();
        editor.add_subtitle(Subtitle::new(
            TimePosition::from_seconds(1.0),
            TimePosition::from_seconds(3.0),
            "Test"
        ).with_id("1"));
        
        assert_eq!(editor.get_track().len(), 1);
        
        // Create new should clear the track
        editor.create_new(SubtitleFormat::WebVtt);
        assert_eq!(editor.get_track().len(), 0);
        assert_eq!(editor.get_format(), SubtitleFormat::WebVtt);
        assert!(editor.is_modified());
    }
} 