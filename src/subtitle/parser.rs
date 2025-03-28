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
        .map_err(|e| Error::parse_error(path_buf, format!("Failed to read file: {}", e)))?;

    if let Some(format) = format {
        match format {
            SubtitleFormat::Srt => parse_srt(&content),
            SubtitleFormat::WebVtt | SubtitleFormat::Vtt => parse_vtt(&content),
            SubtitleFormat::Ass | SubtitleFormat::AdvancedSsa => parse_ssa(&content),
            SubtitleFormat::SubViewer => parse_subviewer(&content),
            SubtitleFormat::MicroDVD => Err(Error::unsupported_parser_format("MicroDVD")),
        }
    } else {
        // Try to detect format from content
        let format = match SubtitleFormat::detect_from_content(&content) {
            Ok(fmt) => fmt,
            Err(_) => {
                return Err(Error::parse_error_with_reason(
                    "Could not determine subtitle format from content",
                ));
            }
        };

        match format {
            SubtitleFormat::Srt => parse_srt(&content),
            SubtitleFormat::WebVtt | SubtitleFormat::Vtt => parse_vtt(&content),
            SubtitleFormat::Ass | SubtitleFormat::AdvancedSsa => parse_ssa(&content),
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

/// Parses an SRT (SubRip) subtitle file.
///
/// # Arguments
///
/// * `path` - Path to the SRT file
///
/// # Returns
///
/// * `Result<SubtitleTrack>` - A subtitle track containing the parsed subtitles
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The file content is invalid SRT format
pub fn parse_srt_file<P: AsRef<Path>>(path: P) -> Result<SubtitleTrack> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<io::Result<Vec<String>>>()?;

    let mut track = SubtitleTrack::new();
    let mut index = 0;

    while index < lines.len() {
        // Skip empty lines
        if lines[index].trim().is_empty() {
            index += 1;
            continue;
        }

        // Parse subtitle number
        let subtitle_number = match lines[index].trim().parse::<usize>() {
            Ok(num) => num,
            Err(_) => return Err(Error::invalid_subtitle_format("Expected subtitle number")),
        };
        index += 1;

        // Parse time codes
        if index >= lines.len() {
            return Err(Error::invalid_subtitle_format("Unexpected end of file"));
        }

        let time_line = &lines[index];
        let times: Vec<&str> = time_line.split("-->").collect();
        if times.len() != 2 {
            return Err(Error::invalid_subtitle_format("Invalid time format"));
        }

        let start_time = TimePosition::from_srt_string(times[0].trim())?;
        let end_time = TimePosition::from_srt_string(times[1].trim())?;
        index += 1;

        // Parse text content (until empty line or end of file)
        if index >= lines.len() {
            return Err(Error::invalid_subtitle_format("Unexpected end of file"));
        }

        let mut text = String::new();
        while index < lines.len() && !lines[index].trim().is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&lines[index]);
            index += 1;
        }

        // Create and add subtitle
        let subtitle =
            Subtitle::new(start_time, end_time, text).with_id(subtitle_number.to_string());
        track.add_subtitle(subtitle);

        // Skip any empty lines
        while index < lines.len() && lines[index].trim().is_empty() {
            index += 1;
        }
    }

    track.sort();
    Ok(track)
}

/// Parses a WebVTT subtitle file.
///
/// # Arguments
///
/// * `path` - Path to the WebVTT file
///
/// # Returns
///
/// * `Result<SubtitleTrack>` - A subtitle track containing the parsed subtitles
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The file content is invalid WebVTT format
pub fn parse_webvtt_file<P: AsRef<Path>>(path: P) -> Result<SubtitleTrack> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<io::Result<Vec<String>>>()?;

    let mut track = SubtitleTrack::new();
    let mut index = 0;

    // Check for WebVTT header
    if lines.is_empty() || !lines[0].trim().starts_with("WEBVTT") {
        return Err(Error::invalid_subtitle_format("Missing WebVTT header"));
    }
    index += 1;

    // Skip header until we find an empty line
    while index < lines.len() && !lines[index].trim().is_empty() {
        index += 1;
    }

    // Skip any empty lines
    while index < lines.len() && lines[index].trim().is_empty() {
        index += 1;
    }

    let mut subtitle_count = 0;
    while index < lines.len() {
        // Skip empty lines and comments
        if lines[index].trim().is_empty() || lines[index].trim().starts_with("NOTE ") {
            index += 1;
            continue;
        }

        // Check if this line is a cue identifier or a timestamp
        let mut cue_id = String::new();
        let timestamp_line;

        if index + 1 < lines.len() && lines[index + 1].contains("-->") {
            // This line is a cue identifier
            cue_id = lines[index].trim().to_string();
            index += 1;
            timestamp_line = &lines[index];
        } else if lines[index].contains("-->") {
            // This line is already a timestamp
            timestamp_line = &lines[index];
        } else {
            // Invalid format
            return Err(Error::invalid_subtitle_format("Expected timestamp line"));
        }

        // Parse timestamps
        let times: Vec<&str> = timestamp_line.split("-->").collect();
        if times.len() != 2 {
            return Err(Error::invalid_subtitle_format("Invalid time format"));
        }

        let (time_part, settings_part) = match times[1].find(' ') {
            Some(pos) => times[1].split_at(pos),
            None => (times[1], ""),
        };

        let start_time = TimePosition::from_vtt_string(times[0].trim())?;
        let end_time = TimePosition::from_vtt_string(time_part.trim())?;

        // TODO: Parse cue settings from settings_part
        let _settings = settings_part.trim();

        index += 1;

        // Parse text content (until empty line or end of file)
        if index >= lines.len() {
            return Err(Error::invalid_subtitle_format("Unexpected end of file"));
        }

        let mut text = String::new();
        while index < lines.len() && !lines[index].trim().is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&lines[index]);
            index += 1;
        }

        // Create and add subtitle
        subtitle_count += 1;
        let id = if cue_id.is_empty() {
            subtitle_count.to_string()
        } else {
            cue_id
        };

        let subtitle = Subtitle::new(start_time, end_time, text).with_id(id);
        track.add_subtitle(subtitle);

        // Skip any empty lines
        while index < lines.len() && lines[index].trim().is_empty() {
            index += 1;
        }
    }

    track.sort();
    Ok(track)
}

/// Parses SRT format subtitle content.
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

    Ok(track)
}

/// Parses an individual SRT entry.
///
/// # Errors
///
/// Returns an error if the entry can't be parsed
fn parse_srt_entry(id: &str, timing: &str, text: &str) -> Result<Subtitle> {
    // Parse the timing line
    let times: Vec<&str> = timing.split(" --> ").collect();
    if times.len() != 2 {
        return Err(Error::parse_error_with_reason(format!(
            "Invalid timing format: {}",
            timing
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
/// A Result containing the parsed TimePosition or an error
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
            "Invalid time format: {}",
            time_str
        )));
    }

    let hours = parts[0].parse::<u32>().map_err(|_| {
        Error::parse_error_with_reason(format!("Invalid hours in time format: {}", parts[0]))
    })?;

    let minutes = parts[1].parse::<u32>().map_err(|_| {
        Error::parse_error_with_reason(format!("Invalid minutes in time format: {}", parts[1]))
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
    if hours >= 24 {
        return Err(Error::parse_error_with_reason(format!(
            "Hours out of range: {}",
            hours
        )));
    }
    if minutes >= 60 {
        return Err(Error::parse_error_with_reason(format!(
            "Minutes out of range: {}",
            minutes
        )));
    }
    if seconds >= 60 {
        return Err(Error::parse_error_with_reason(format!(
            "Seconds out of range: {}",
            seconds
        )));
    }
    if milliseconds >= 1000 {
        return Err(Error::parse_error_with_reason(format!(
            "Milliseconds out of range: {}",
            milliseconds
        )));
    }

    Ok(TimePosition::new(hours, minutes, seconds, milliseconds))
}

/// Parses WebVTT format subtitle content.
///
/// # Errors
///
/// Returns an error if parsing fails
fn parse_vtt(content: &str) -> Result<SubtitleTrack> {
    let mut track = SubtitleTrack::new();
    let mut lines = content.lines();

    // Skip the WEBVTT header
    let mut header_found = false;
    for line in lines.by_ref() {
        if line.trim().starts_with("WEBVTT") {
            header_found = true;
            break;
        }
    }

    if !header_found {
        return Err(Error::parse_error_with_reason("Missing WEBVTT header"));
    }

    // Process the rest of the file
    let mut current_id = String::new();
    let mut current_timing = String::new();
    let mut current_text = Vec::new();
    let mut in_subtitle = false;
    let mut subtitle_count = 0;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_subtitle && !current_text.is_empty() {
                // End of a subtitle entry
                let subtitle =
                    parse_vtt_entry(&current_id, &current_timing, &current_text.join("\n"))?;
                track.add_subtitle(subtitle);

                // Reset for next entry
                current_id.clear();
                current_timing.clear();
                current_text.clear();
                in_subtitle = false;
            }
        } else if trimmed.contains(" --> ") {
            // This is a timing line
            current_timing = trimmed.to_string();
            in_subtitle = true;
            subtitle_count += 1;

            // If we don't have an ID, use the subtitle count
            if current_id.is_empty() {
                current_id = subtitle_count.to_string();
            }
        } else if in_subtitle {
            // This is part of the text content
            current_text.push(trimmed.to_string());
        } else {
            // This might be a cue identifier
            current_id = trimmed.to_string();
        }
    }

    // Add the last subtitle if there is one
    if in_subtitle && !current_text.is_empty() {
        let subtitle = parse_vtt_entry(&current_id, &current_timing, &current_text.join("\n"))?;
        track.add_subtitle(subtitle);
    }

    Ok(track)
}

/// Parses an individual WebVTT entry.
///
/// # Errors
///
/// Returns an error if the entry can't be parsed
fn parse_vtt_entry(id: &str, timing: &str, text: &str) -> Result<Subtitle> {
    // Parse the timing line
    let times: Vec<&str> = timing.split(" --> ").collect();
    if times.len() != 2 {
        return Err(Error::parse_error_with_reason(format!(
            "Invalid timing format: {}",
            timing
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

/// Parses a time string in WebVTT format (00:00:00.000).
///
/// # Errors
///
/// Returns an error if the time can't be parsed
fn parse_vtt_time(time_str: &str) -> Result<TimePosition> {
    let parts: Vec<&str> = time_str.split(':').collect();

    if parts.len() == 3 {
        // Format is HH:MM:SS.mmm
        let hours: u32 = parts[0]
            .parse()
            .map_err(|_| Error::parse_error_with_reason(format!("Invalid hours: {}", parts[0])))?;

        let minutes: u32 = parts[1].parse().map_err(|_| {
            Error::parse_error_with_reason(format!("Invalid minutes: {}", parts[1]))
        })?;

        let seconds_parts: Vec<&str> = parts[2].split('.').collect();
        let seconds: u32 = seconds_parts[0].parse().map_err(|_| {
            Error::parse_error_with_reason(format!("Invalid seconds: {}", seconds_parts[0]))
        })?;

        let milliseconds = if seconds_parts.len() > 1 {
            let mut ms_str = seconds_parts[1].to_string();
            while ms_str.len() < 3 {
                ms_str.push('0');
            }
            ms_str.truncate(3);
            ms_str.parse().unwrap_or(0)
        } else {
            0
        };

        Ok(TimePosition::new(hours, minutes, seconds, milliseconds))
    } else if parts.len() == 2 {
        // Format is MM:SS.mmm
        let minutes: u32 = parts[0].parse().map_err(|_| {
            Error::parse_error_with_reason(format!("Invalid minutes: {}", parts[0]))
        })?;

        let seconds_parts: Vec<&str> = parts[1].split('.').collect();
        let seconds: u32 = seconds_parts[0].parse().map_err(|_| {
            Error::parse_error_with_reason(format!("Invalid seconds: {}", seconds_parts[0]))
        })?;

        let milliseconds = if seconds_parts.len() > 1 {
            let mut ms_str = seconds_parts[1].to_string();
            while ms_str.len() < 3 {
                ms_str.push('0');
            }
            ms_str.truncate(3);
            ms_str.parse().unwrap_or(0)
        } else {
            0
        };

        Ok(TimePosition::new(0, minutes, seconds, milliseconds))
    } else {
        Err(Error::parse_error_with_reason(format!(
            "Invalid time format: {}",
            time_str
        )))
    }
}

/// Stub for parsing SSA/ASS format subtitles.
///
/// # Errors
///
/// Returns an unsupported format error
fn parse_ssa(_content: &str) -> Result<SubtitleTrack> {
    Err(Error::unsupported_parser_format("ASS/SSA"))
}

/// Stub for parsing SubViewer format subtitles.
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

        assert_eq!(subtitles[0].get_id(), "1");
        assert_eq!(subtitles[0].get_start().as_seconds(), 1.0);
        assert_eq!(subtitles[0].get_end().as_seconds(), 4.0);
        assert_eq!(subtitles[0].get_text(), "This is the first subtitle");

        assert_eq!(subtitles[1].get_id(), "cue-2");
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
