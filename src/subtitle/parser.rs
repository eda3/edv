/// Parser implementations for various subtitle formats.
///
/// This module provides parsers for different subtitle formats,
/// converting raw subtitle file content into structured subtitle tracks.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use crate::subtitle::error::{Error, Result};
use crate::subtitle::format::{SubtitleFormat, TimePosition};
use crate::subtitle::model::{Subtitle, SubtitleTrack};

/// Parses a subtitle file in any supported format.
///
/// This function detects the format of the subtitle file and
/// calls the appropriate parser based on the detected format.
///
/// # Arguments
///
/// * `path` - Path to the subtitle file
///
/// # Returns
///
/// * `Result<SubtitleTrack>` - A subtitle track containing the parsed subtitles
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The format cannot be detected
/// * The file content is invalid for the detected format
pub fn parse_subtitle_file<P: AsRef<Path>>(path: P) -> Result<SubtitleTrack> {
    let path = path.as_ref();
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    // Try to determine format from extension
    let format = match extension.as_deref() {
        Some("srt") => SubtitleFormat::Srt,
        Some("vtt") => SubtitleFormat::WebVtt,
        Some("ass" | "ssa") => SubtitleFormat::AdvancedSsa,
        Some("sub") => {
            // Check if it's MicroDVD or SubViewer by looking at first line
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);
            let mut first_line = String::new();
            reader.read_line(&mut first_line)?;
            
            if first_line.starts_with('{') && first_line.contains('}') {
                SubtitleFormat::MicroDvd
            } else {
                SubtitleFormat::SubViewer
            }
        }
        _ => {
            // Try to detect format from content
            detect_format_from_content(path)?
        }
    };
    
    // Parse based on detected format
    match format {
        SubtitleFormat::Srt => parse_srt_file(path),
        SubtitleFormat::WebVtt => parse_webvtt_file(path),
        SubtitleFormat::AdvancedSsa => Err(Error::unsupported_parser_format("ASS/SSA")),
        SubtitleFormat::SubViewer => Err(Error::unsupported_parser_format("SubViewer")),
        SubtitleFormat::MicroDvd => Err(Error::unsupported_parser_format("MicroDVD")),
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
            let has_timestamp = next_line.contains("-->") && 
                                (next_line.contains(':') && next_line.contains(','));
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
    if lines.iter().any(|line| line.starts_with('{') && line.contains("}{")) {
        return Ok(SubtitleFormat::MicroDvd);
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
    let lines: Vec<String> = reader
        .lines()
        .collect::<io::Result<Vec<String>>>()?;
    
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
        let subtitle = Subtitle::new(start_time, end_time, text)
            .with_id(subtitle_number.to_string());
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
    let lines: Vec<String> = reader
        .lines()
        .collect::<io::Result<Vec<String>>>()?;
    
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
        
        let subtitle = Subtitle::new(start_time, end_time, text)
            .with_id(id);
        track.add_subtitle(subtitle);
        
        // Skip any empty lines
        while index < lines.len() && lines[index].trim().is_empty() {
            index += 1;
        }
    }
    
    track.sort();
    Ok(track)
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
        assert_eq!(subtitles[1].get_text(), "This is the second subtitle\nIt has multiple lines");
        
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
        assert_eq!(subtitles[1].get_text(), "This is the second subtitle\nIt has multiple lines");
        
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
        
        let track = parse_subtitle_file(&new_path).unwrap();
        assert_eq!(track.len(), 3);
        
        // WebVTT with .vtt extension
        let vtt_file = create_temp_webvtt_file();
        let path = vtt_file.path().to_owned();
        let new_path = path.with_extension("vtt");
        fs::rename(&path, &new_path).unwrap();
        
        let track = parse_subtitle_file(&new_path).unwrap();
        assert_eq!(track.len(), 3);
    }
} 