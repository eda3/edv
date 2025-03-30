use std::fs::{self, File};
use std::io;
use std::io::{BufRead, BufReader};
/// Parser implementations for various subtitle formats.
///
/// This module provides parsers for different subtitle formats,
/// converting raw subtitle file content into structured subtitle tracks.
use std::path::Path;

use crate::subtitle::error::{Error, Result};
use crate::subtitle::format::{SubtitleFormat, TimePosition};
use crate::subtitle::model::{Subtitle, SubtitleTrack};

/// Attempts to parse a subtitle file.
///
/// Based on the format, it will use the appropriate parser.
///
/// # Errors
///
/// Returns an error if the parser fails or the format is not supported
pub fn parse_subtitle_file<P: AsRef<Path>>(
    path: P,
    format: Option<SubtitleFormat>,
) -> Result<SubtitleTrack> {
    let path_buf = path.as_ref().to_path_buf();
    let content = fs::read_to_string(&path)
        .map_err(|e| Error::parse_error(path_buf, format!("Failed to read file: {e}")))?;

    if let Some(format) = format {
        match format {
            SubtitleFormat::Srt => parse_srt(&content),
            SubtitleFormat::WebVtt => parse_vtt(&content),
            SubtitleFormat::AdvancedSsa => parse_ssa(&content),
            SubtitleFormat::SubViewer => parse_subviewer(&content),
            SubtitleFormat::MicroDVD => Err(Error::unsupported_parser_format("MicroDVD")),
        }
    } else {
        // Try to detect format from content
        let Ok(format) = SubtitleFormat::detect_from_content(&content) else {
            return Err(Error::parse_error_with_reason(
                "Could not determine subtitle format from content",
            ));
        };

        match format {
            SubtitleFormat::Srt => parse_srt(&content),
            SubtitleFormat::WebVtt => parse_vtt(&content),
            SubtitleFormat::AdvancedSsa => parse_ssa(&content),
            SubtitleFormat::SubViewer => parse_subviewer(&content),
            SubtitleFormat::MicroDVD => Err(Error::unsupported_parser_format("MicroDVD")),
        }
    }
}

/// Detects subtitle format by examining file content.
///
/// # Arguments
///
/// * `path` - Path to the subtitle file
///
/// # Returns
///
/// * `Result<SubtitleFormat>` - The detected subtitle format
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The format cannot be determined
#[allow(dead_code)]
fn detect_format_from_content<P: AsRef<Path>>(path: P) -> Result<SubtitleFormat> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .take(10) // Only check first 10 lines
        .collect::<io::Result<Vec<String>>>()?;

    // Check for WebVTT header
    if lines.iter().any(|line| line.trim().starts_with("WEBVTT")) {
        return Ok(SubtitleFormat::WebVtt);
    }

    // Check for SRT format (numbered entries, followed by timestamps with '00:00:00,000 --> 00:00:00,000' format)
    let is_srt = lines.iter().enumerate().any(|(i, line)| {
        if i + 2 < lines.len() {
            // Look for lines that have just a number, followed by a timestamp line
            let is_number = line.trim().parse::<usize>().is_ok();
            let next_line = &lines[i + 1];
            let has_timestamp =
                next_line.contains("-->") && (next_line.contains(':') && next_line.contains(','));
            return is_number && has_timestamp;
        }
        false
    });

    if is_srt {
        return Ok(SubtitleFormat::Srt);
    }

    // Check for ASS/SSA format
    if lines.iter().any(|line| line.contains("[Script Info]")) {
        return Ok(SubtitleFormat::AdvancedSsa);
    }

    // Check for MicroDVD format (lines starting with {frame_number})
    if lines
        .iter()
        .any(|line| line.starts_with('{') && line.contains("}{"))
    {
        return Ok(SubtitleFormat::MicroDVD);
    }

    // Default to SRT if we couldn't determine
    Err(Error::unknown_subtitle_format())
}

/// Parses an `SRT` subtitle file.
///
/// # Arguments
///
/// * `path` - Path to the `SRT` file
///
/// # Returns
///
/// * `Result<SubtitleTrack>` - A subtitle track containing the parsed subtitles
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The file content is invalid `SRT` format
pub fn parse_srt_file<P: AsRef<Path>>(path: P) -> Result<SubtitleTrack> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let content = reader
        .lines()
        .collect::<io::Result<Vec<String>>>()?
        .join("\n");

    parse_srt(&content)
}

/// Parses `SRT` format subtitle content.
///
/// # Errors
///
/// Returns an error if parsing fails
fn parse_srt(content: &str) -> Result<SubtitleTrack> {
    let mut track = SubtitleTrack::new();
    let mut current_id = String::new();
    let mut current_timing = String::new();
    let mut current_text = Vec::new();
    let mut in_subtitle = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_subtitle && !current_text.is_empty() {
                // End of a subtitle entry
                let subtitle =
                    parse_srt_entry(&current_id, &current_timing, &current_text.join("\n"))?;
                track.add_subtitle(subtitle);

                // Reset for next entry
                current_id.clear();
                current_timing.clear();
                current_text.clear();
                in_subtitle = false;
            }
        } else if !in_subtitle {
            // Start of a new subtitle - this should be the ID
            current_id = trimmed.to_string();
            in_subtitle = true;
        } else if current_timing.is_empty() {
            // This should be the timing line
            current_timing = trimmed.to_string();
        } else {
            // This is part of the text content
            current_text.push(trimmed.to_string());
        }
    }

    // Add the last subtitle if there is one
    if in_subtitle && !current_text.is_empty() {
        let subtitle = parse_srt_entry(&current_id, &current_timing, &current_text.join("\n"))?;
        track.add_subtitle(subtitle);
    }

    track.sort();
    Ok(track)
}

/// Parses an individual `SRT` entry.
///
/// # Errors
///
/// Returns an error if the entry can't be parsed
fn parse_srt_entry(id: &str, timing: &str, text: &str) -> Result<Subtitle> {
    // Parse the timing line
    let times: Vec<&str> = timing.split(" --> ").collect();
    if times.len() != 2 {
        return Err(Error::parse_error_with_reason(format!(
            "Invalid timing format: {timing}"
        )));
    }

    let start = parse_time(times[0].trim())?;
    let end = parse_time(times[1].trim())?;

    // Create the subtitle
    Ok(Subtitle::new(start, end, text).with_id(id))
}

/// Parses a time position from a string with a comma (,) separator between seconds and milliseconds.
///
/// # Arguments
///
/// * `time_str` - The time string to parse
///
/// # Returns
///
/// A Result containing the parsed `TimePosition` or an error
///
/// # Errors
///
/// Returns an error if the time can't be parsed
fn parse_time(time_str: &str) -> Result<TimePosition> {
    // 置換した結果を一時変数に保存して生存期間を延長する
    let time_with_dots = time_str.replace(',', ".");
    let parts: Vec<&str> = time_with_dots.split(':').collect();
    if parts.len() != 3 {
        return Err(Error::parse_error_with_reason(format!(
            "Invalid time format: {time_str}"
        )));
    }

    let hours = parts[0].parse::<u32>().map_err(|e| {
        Error::parse_error_with_reason(format!("Invalid hours in time format: {e}"))
    })?;

    let minutes = parts[1].parse::<u32>().map_err(|e| {
        Error::parse_error_with_reason(format!("Invalid minutes in time format: {e}"))
    })?;

    // 秒とミリ秒を分ける
    let seconds_parts: Vec<&str> = parts[2].split('.').collect();
    if seconds_parts.len() != 2 {
        return Err(Error::parse_error_with_reason(format!(
            "Invalid seconds format: {}",
            parts[2]
        )));
    }

    let seconds = seconds_parts[0].parse::<u32>().map_err(|_| {
        Error::parse_error_with_reason(format!(
            "Invalid seconds in time format: {}",
            seconds_parts[0]
        ))
    })?;

    let milliseconds = seconds_parts[1].parse::<u32>().map_err(|_| {
        Error::parse_error_with_reason(format!(
            "Invalid milliseconds in time format: {}",
            seconds_parts[1]
        ))
    })?;

    // Validate times
    if hours > 99 {
        return Err(Error::parse_error_with_reason(format!(
            "Hours out of range: {hours}"
        )));
    }
    if minutes > 59 {
        return Err(Error::parse_error_with_reason(format!(
            "Minutes out of range: {minutes}"
        )));
    }
    if seconds > 59 {
        return Err(Error::parse_error_with_reason(format!(
            "Seconds out of range: {seconds}"
        )));
    }
    if milliseconds > 999 {
        return Err(Error::parse_error_with_reason(format!(
            "Milliseconds out of range: {milliseconds}"
        )));
    }

    Ok(TimePosition::new(hours, minutes, seconds, milliseconds))
}

/// Parses a `WebVTT` subtitle file.
///
/// # Arguments
///
/// * `path` - Path to the `WebVTT` file
///
/// # Returns
///
/// * `Result<SubtitleTrack>` - A subtitle track containing the parsed subtitles
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The file content is invalid `WebVTT` format
pub fn parse_webvtt_file<P: AsRef<Path>>(path: P) -> Result<SubtitleTrack> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let content = reader
        .lines()
        .collect::<io::Result<Vec<String>>>()?
        .join("\n");

    parse_vtt(&content)
}

/// Parses `WebVTT` format subtitle content.
///
/// # Errors
///
/// Returns an error if parsing fails
fn parse_vtt(content: &str) -> Result<SubtitleTrack> {
    let mut track = SubtitleTrack::new();
    let mut lines = content.lines();

    // Skip "WEBVTT" header and potential blank lines
    // Find the start of cues, handling potential initial comments/styles
    let mut line_iter = lines.skip_while(|line| line.trim().is_empty() || line.trim() == "WEBVTT");

    let mut current_id: Option<String> = None;
    let mut current_timing: Option<String> = None;
    let mut current_text: Vec<String> = Vec::new();
    let mut processing_cue = false;
    let mut unnamed_cue_counter = 1; // Counter for cues without explicit IDs

    // Process lines using the iterator
    while let Some(line) = line_iter.next() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Blank line signifies the end of a cue block or separates cues.
            if processing_cue && current_timing.is_some() {
                // Finalize the previous cue
                let id_to_use = match current_id.take() {
                    Some(id) if !id.is_empty() => id,
                    _ => {
                        // Generate ID if None or empty
                        let generated_id = unnamed_cue_counter.to_string();
                        unnamed_cue_counter += 1;
                        generated_id
                    }
                };

                let timing = current_timing.take().unwrap_or_default();
                let text = current_text.join("\n");
                let subtitle = parse_vtt_entry(&id_to_use, &timing, &text)?;
                track.add_subtitle(subtitle);

                // Reset for the next cue
                current_text.clear();
                processing_cue = false; // Finished processing this cue
            }
            continue; // Skip blank lines otherwise
        }

        if trimmed.starts_with("NOTE") || trimmed.starts_with("STYLE") {
            // Skip comments and style blocks
            // Handle potential multi-line blocks if needed later
            // Ensure state is reset if a comment interrupts a cue definition
            if !processing_cue {
                current_id = None;
            }
            continue;
        }

        // If it contains "-->", it's a timing line
        if trimmed.contains("-->") {
            // If timing appears before text, finalize any preceding ID-only line
            // This handles cases like ID\nTIMING\nTEXT
            current_timing = Some(trimmed.to_string());
            processing_cue = true; // Start processing this cue block (ID or not)
        } else if processing_cue {
            // If we are processing a cue (have seen timing), this line is text
            current_text.push(trimmed.to_string());
        } else {
            // If not processing a cue and it's not blank/comment/timing,
            // it must be a cue identifier.
            current_id = Some(trimmed.to_string());
            // Reset text buffer in case of ID -> ID sequences
            current_text.clear();
        }
    }

    // Add the last subtitle if buffer is not empty
    if processing_cue && current_timing.is_some() {
        let id_to_use = match current_id.take() {
            Some(id) if !id.is_empty() => id,
            _ => {
                let generated_id = unnamed_cue_counter.to_string();
                // unnamed_cue_counter += 1; // No need to increment after loop
                generated_id
            }
        };
        let timing = current_timing.unwrap_or_default();
        let text = current_text.join("\n");
        let subtitle = parse_vtt_entry(&id_to_use, &timing, &text)?;
        track.add_subtitle(subtitle);
    }

    track.sort();
    Ok(track)
}

/// Parses an individual `WebVTT` entry.
///
/// # Errors
///
/// Returns an error if the entry can't be parsed
fn parse_vtt_entry(id: &str, timing: &str, text: &str) -> Result<Subtitle> {
    // Parse the timing line
    let times: Vec<&str> = timing.split(" --> ").collect();
    if times.len() != 2 {
        return Err(Error::parse_error_with_reason(format!(
            "Invalid timing format: {timing}"
        )));
    }

    let start_time = times[0].trim();
    let end_parts: Vec<&str> = times[1].split_whitespace().collect();
    let end_time = end_parts[0];

    let start = parse_vtt_time(start_time)?;
    let end = parse_vtt_time(end_time)?;

    // Create the subtitle
    Ok(Subtitle::new(start, end, text).with_id(id))
}

/// Parses a time string in `WebVTT` format (00:00:00.000).
///
/// # Errors
///
/// Returns an error if the time can't be parsed
fn parse_vtt_time(time_str: &str) -> Result<TimePosition> {
    if time_str.is_empty() {
        return Err(Error::timing_error(
            "Cannot parse empty string as VTT time".to_string(),
        ));
    }

    let parts: Vec<&str> = time_str.split(':').collect();
    let (hours, minutes, seconds_str) = match parts.len() {
        3 => (
            parts[0]
                .parse::<u32>()
                .map_err(|_| Error::timing_error(format!("Invalid VTT hours: {}", parts[0])))?,
            parts[1]
                .parse::<u32>()
                .map_err(|_| Error::timing_error(format!("Invalid VTT minutes: {}", parts[1])))?,
            parts[2],
        ),
        2 => (
            0,
            parts[0]
                .parse::<u32>()
                .map_err(|_| Error::timing_error(format!("Invalid VTT minutes: {}", parts[0])))?,
            parts[1],
        ),
        _ => {
            return Err(Error::timing_error(format!(
                "Invalid VTT time format (parts): {time_str}"
            )));
        }
    };

    // Split seconds and milliseconds more robustly
    let seconds: u32;
    let milliseconds: u32;
    if let Some((sec_str, ms_str)) = seconds_str.split_once('.') {
        seconds = sec_str
            .parse()
            .map_err(|_| Error::timing_error(format!("Invalid VTT seconds: {}", sec_str)))?;
        // Parse milliseconds, handling padding and truncation
        let mut ms_padded = ms_str.to_string();
        if ms_padded.is_empty() {
            // Handle case like "SS." -> ms = 0
            milliseconds = 0;
        } else {
            while ms_padded.len() < 3 {
                ms_padded.push('0');
            }
            ms_padded.truncate(3);
            milliseconds = ms_padded.parse().map_err(|_| {
                Error::timing_error(format!("Invalid VTT milliseconds: {}", ms_str))
            })?;
        }
    } else {
        // No milliseconds part found
        seconds = seconds_str
            .parse()
            .map_err(|_| Error::timing_error(format!("Invalid VTT seconds: {}", seconds_str)))?;
        milliseconds = 0;
    }

    // Validate components
    if minutes > 59 || seconds > 59
    // milliseconds > 999 // Should be handled by parsing logic now
    // VTT allows large hours, and MM:SS minutes check is implicit if seconds/ms parsed ok
    {
        return Err(Error::timing_error(format!(
            "VTT time component out of range (MM/SS): {time_str}"
        )));
    }

    Ok(TimePosition::new(hours, minutes, seconds, milliseconds))
}

/// Stub for parsing SSA/ASS format subtitles.
///
/// # Errors
///
/// Returns an unsupported format error
fn parse_ssa(_content: &str) -> Result<SubtitleTrack> {
    Err(Error::unsupported_parser_format("ASS/SSA"))
}

/// Stub for parsing `SubViewer` format subtitles.
///
/// # Errors
///
/// Returns an unsupported format error
fn parse_subviewer(_content: &str) -> Result<SubtitleTrack> {
    Err(Error::unsupported_parser_format("SubViewer"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_srt_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "1").unwrap();
        writeln!(file, "00:00:01,000 --> 00:00:04,000").unwrap();
        writeln!(file, "This is the first subtitle").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "2").unwrap();
        writeln!(file, "00:00:05,500 --> 00:00:07,500").unwrap();
        writeln!(file, "This is the second subtitle").unwrap();
        writeln!(file, "It has multiple lines").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "3").unwrap();
        writeln!(file, "00:01:00,000 --> 00:01:30,000").unwrap();
        writeln!(file, "This is the third subtitle").unwrap();
        file.flush().unwrap();
        file
    }

    fn create_temp_webvtt_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "WEBVTT").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "00:00:01.000 --> 00:00:04.000").unwrap();
        writeln!(file, "This is the first subtitle").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "cue-2").unwrap();
        writeln!(file, "00:00:05.500 --> 00:00:07.500 align:right").unwrap();
        writeln!(file, "This is the second subtitle").unwrap();
        writeln!(file, "It has multiple lines").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "NOTE This is a comment and should be ignored").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "00:01:00.000 --> 00:01:30.000").unwrap();
        writeln!(file, "This is the third subtitle").unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_srt_parsing() {
        let file = create_temp_srt_file();
        let track = parse_srt_file(file.path()).unwrap();

        assert_eq!(track.len(), 3);

        let subtitles = track.get_subtitles();

        assert_eq!(subtitles[0].get_id(), "1");
        assert_eq!(subtitles[0].get_start().as_seconds(), 1.0);
        assert_eq!(subtitles[0].get_end().as_seconds(), 4.0);
        assert_eq!(subtitles[0].get_text(), "This is the first subtitle");

        assert_eq!(subtitles[1].get_id(), "2");
        assert_eq!(subtitles[1].get_start().as_seconds(), 5.5);
        assert_eq!(subtitles[1].get_end().as_seconds(), 7.5);
        assert_eq!(
            subtitles[1].get_text(),
            "This is the second subtitle\nIt has multiple lines"
        );

        assert_eq!(subtitles[2].get_id(), "3");
        assert_eq!(subtitles[2].get_start().as_seconds(), 60.0);
        assert_eq!(subtitles[2].get_end().as_seconds(), 90.0);
        assert_eq!(subtitles[2].get_text(), "This is the third subtitle");
    }

    #[test]
    fn test_webvtt_parsing() {
        let file = create_temp_webvtt_file();
        let track = parse_webvtt_file(file.path()).unwrap();

        assert_eq!(track.len(), 3);

        let subtitles = track.get_subtitles();

        // Check first cue (no ID in file -> generated ID "1")
        assert_eq!(subtitles[0].get_id(), "1");
        assert_eq!(subtitles[0].get_start().as_seconds(), 1.0);
        assert_eq!(subtitles[0].get_end().as_seconds(), 4.0);
        assert_eq!(subtitles[0].get_text(), "This is the first subtitle");

        // Check second cue (explicit ID "cue-2")
        assert_eq!(subtitles[1].get_id(), "cue-2");
        assert_eq!(subtitles[1].get_start().as_seconds(), 5.5);
        assert_eq!(subtitles[1].get_end().as_seconds(), 7.5);
        assert_eq!(
            subtitles[1].get_text(),
            "This is the second subtitle\nIt has multiple lines"
        );

        // Check third cue (no ID in file -> generated ID "2")
        assert_eq!(subtitles[2].get_id(), "2");
        assert_eq!(subtitles[2].get_start().as_seconds(), 60.0);
        assert_eq!(subtitles[2].get_end().as_seconds(), 90.0);
        assert_eq!(subtitles[2].get_text(), "This is the third subtitle");
    }

    #[test]
    fn test_format_detection() {
        // SRT
        let srt_file = create_temp_srt_file();
        let format = detect_format_from_content(srt_file.path()).unwrap();
        assert_eq!(format, SubtitleFormat::Srt);

        // WebVTT
        let vtt_file = create_temp_webvtt_file();
        let format = detect_format_from_content(vtt_file.path()).unwrap();
        assert_eq!(format, SubtitleFormat::WebVtt);
    }

    #[test]
    fn test_parse_any_subtitle() {
        // SRT with .srt extension
        let srt_file = create_temp_srt_file();
        let path = srt_file.path().to_owned();
        let new_path = path.with_extension("srt");
        fs::rename(&path, &new_path).unwrap();

        let track = parse_subtitle_file(&new_path, None).unwrap();
        assert_eq!(track.len(), 3);

        // WebVTT with .vtt extension
        let vtt_file = create_temp_webvtt_file();
        let path = vtt_file.path().to_owned();
        let new_path = path.with_extension("vtt");
        fs::rename(&path, &new_path).unwrap();

        let track = parse_subtitle_file(&new_path, None).unwrap();
        assert_eq!(track.len(), 3);
    }
}
