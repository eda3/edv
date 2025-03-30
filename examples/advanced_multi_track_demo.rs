/// Advanced Multi-track Editing Sample Project
///
/// This sample demonstrates creating a complex editing project with multiple video and audio tracks,
/// utilizing track visibility and locking features.
///
/// # How to Run
///
/// ```bash
/// # When running in a WSL environment, set the TMPDIR environment variable
/// # First, create a temporary directory
/// mkdir -p output/temp
/// chmod -R 1777 output
///
/// # Then, run with the environment variable set
/// TMPDIR=$(pwd)/output/temp cargo run --example advanced_multi_track_demo
/// ```
///
/// # TODO
///
/// * Fix temporary directory creation issues in WSL environment
/// * Implement more flexible media file path specification
use std::path::{Path, PathBuf};

use edv::ffmpeg::FFmpeg;
use edv::project::rendering::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
use edv::project::timeline::keyframes::EasingFunction;
use edv::project::timeline::multi_track::TrackRelationship;
use edv::project::timeline::{Clip, TrackKind};
use edv::project::{AssetId, AssetMetadata, AssetReference, ClipId, Project};
use edv::utility::time::{Duration, TimePosition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FFmpeg
    let ffmpeg = FFmpeg::detect()?;
    println!("FFmpeg detected: {}", ffmpeg.path().display());
    println!("FFmpeg version: {}", ffmpeg.version());

    // Create output directory
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create a project
    let mut project = Project::new("Advanced Multi-track Sample");

    // Set input file path
    let input_file = PathBuf::from("test_media/sozai.mp4");
    if !input_file.exists() {
        return Err(format!("Input file not found: {}", input_file.display()).into());
    }

    // Get media information from input file
    let media_info = ffmpeg.get_media_info(&input_file)?;

    // Get basic data from media info
    let main_video_duration = media_info.duration_seconds().unwrap_or(10.0);

    // Get video resolution from media info
    let video_dimensions = if let Some(video_stream) = media_info.video_streams().first() {
        (
            video_stream.width.unwrap_or(1280) as u32,
            video_stream.height.unwrap_or(720) as u32,
        )
    } else {
        (1280, 720) // Default values
    };

    // Create asset metadata
    let mut asset_metadata = AssetMetadata {
        duration: Some(Duration::from_seconds(main_video_duration)),
        dimensions: Some(video_dimensions),
        asset_type: "video".to_string(),
        extra: std::collections::HashMap::new(),
    };

    // Add main asset
    let main_asset_id = project.add_asset(input_file.clone(), asset_metadata.clone());

    // Add the same file as a second asset (for overlay)
    asset_metadata
        .extra
        .insert("purpose".to_string(), "overlay".to_string());
    let overlay_asset_id = project.add_asset(input_file.clone(), asset_metadata.clone());

    // Add the same file as a third asset (for background)
    asset_metadata
        .extra
        .insert("purpose".to_string(), "background".to_string());
    let background_asset_id = project.add_asset(input_file.clone(), asset_metadata);

    // Add tracks to timeline
    let background_track_id = project.timeline.add_track(TrackKind::Video); // Background video
    let main_video_track_id = project.timeline.add_track(TrackKind::Video); // Main video
    let overlay_video_track_id = project.timeline.add_track(TrackKind::Video); // Overlay video
    let main_audio_track_id = project.timeline.add_track(TrackKind::Audio); // Main audio
    let secondary_audio_track_id = project.timeline.add_track(TrackKind::Audio); // Secondary audio for sound effects

    // Set track names
    if let Some(track) = project.timeline.get_track_mut(background_track_id) {
        track.set_name("Background Video");
    }

    if let Some(track) = project.timeline.get_track_mut(main_video_track_id) {
        track.set_name("Main Video");
    }

    if let Some(track) = project.timeline.get_track_mut(overlay_video_track_id) {
        track.set_name("Overlay Video");
    }

    if let Some(track) = project.timeline.get_track_mut(main_audio_track_id) {
        track.set_name("Main Audio");
    }

    if let Some(track) = project.timeline.get_track_mut(secondary_audio_track_id) {
        track.set_name("Sound Effects");
    }

    // Create background video clip (covering the entire timeline)
    let background_clip = Clip::new(
        ClipId::new(),
        background_asset_id,
        TimePosition::from_seconds(0.0),
        Duration::from_seconds(main_video_duration + 2.0), // Slightly longer
        TimePosition::from_seconds(0.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project
        .timeline
        .add_clip(background_track_id, background_clip)?;

    // Main video clip (starting 2 seconds later)
    let main_clip = Clip::new(
        ClipId::new(),
        main_asset_id,
        TimePosition::from_seconds(2.0), // Start after 2 seconds
        Duration::from_seconds(main_video_duration - 2.0), // Slightly shorter
        TimePosition::from_seconds(1.0), // From 1 second in the original video
        TimePosition::from_seconds(main_video_duration),
    );

    project.timeline.add_clip(main_video_track_id, main_clip)?;

    // Overlay video clip (displayed for 3 seconds at the 4 second mark)
    let overlay_clip = Clip::new(
        ClipId::new(),
        overlay_asset_id,
        TimePosition::from_seconds(4.0), // Start after 4 seconds
        Duration::from_seconds(3.0),     // Display for 3 seconds
        TimePosition::from_seconds(2.0), // From 2 seconds in the original video
        TimePosition::from_seconds(5.0), // To 5 seconds in the original video
    );

    project
        .timeline
        .add_clip(overlay_video_track_id, overlay_clip)?;

    // Main audio clip (matching the main video timing)
    let main_audio_clip = Clip::new(
        ClipId::new(),
        main_asset_id, // Use audio from the same asset
        TimePosition::from_seconds(2.0),
        Duration::from_seconds(main_video_duration - 2.0),
        TimePosition::from_seconds(1.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project
        .timeline
        .add_clip(main_audio_track_id, main_audio_clip)?;

    // Sound effect audio clip (repeating a portion of the original video's audio)
    let effect_audio_clip = Clip::new(
        ClipId::new(),
        main_asset_id,                   // Use audio from the same asset
        TimePosition::from_seconds(4.0), // After 4 seconds
        Duration::from_seconds(3.0),     // For 3 seconds
        TimePosition::from_seconds(0.0), // From the beginning of the original video
        TimePosition::from_seconds(3.0), // To 3 seconds in the original video
    );

    project
        .timeline
        .add_clip(secondary_audio_track_id, effect_audio_clip)?;

    // Set track mute and lock states
    if let Some(track) = project.timeline.get_track_mut(secondary_audio_track_id) {
        track.set_muted(true); // Mute the sound effects track (won't be included in rendering)
    }

    if let Some(track) = project.timeline.get_track_mut(background_track_id) {
        track.set_locked(true); // Lock the background track
    }

    // Set background track opacity with keyframes (30%)
    project.timeline.add_keyframe_with_history(
        background_track_id,
        "opacity",
        TimePosition::from_seconds(0.0),
        0.3, // 30% opacity
        EasingFunction::Linear,
    )?;

    // Set overlay video opacity with keyframes (70%)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(0.0),
        0.7, // 70% opacity
        EasingFunction::Linear,
    )?;

    // Add keyframe animations to overlay video
    // Opacity keyframe animation
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(4.0), // Starting point
        0.0,                             // Starting opacity (0%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(5.0), // After 1 second
        0.7,                             // Opacity 70%
        EasingFunction::EaseOut,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(6.0), // After 2 seconds
        0.7,                             // Opacity 70%
        EasingFunction::EaseIn,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(7.0), // After 3 seconds
        0.0,                             // Opacity 0%
        EasingFunction::Linear,
    )?;

    // Main audio volume keyframes (fade in/fade out)
    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(2.0), // Starting point
        0.0,                             // Starting volume (0%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(3.0), // After 1 second
        1.0,                             // Volume 100%
        EasingFunction::EaseOut,
    )?;

    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(main_video_duration - 1.0), // 1 second before end
        1.0,                                                   // Volume 100%
        EasingFunction::EaseIn,
    )?;

    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(main_video_duration), // End point
        0.0,                                             // Volume 0%
        EasingFunction::Linear,
    )?;

    // Set relationships between tracks (linking main video and main audio)
    // Note: Commented out due to complex Rust ownership and lifetime issues
    // This is omitted in the sample
    /*
    {
        // Create a clone to get the multi-track manager
        let timeline_ref = project.timeline.clone();
        let mut multi_track_manager = timeline_ref.multi_track_manager_mut();

        // Set relationships between tracks (linking main video and main audio)
        multi_track_manager.add_relationship(
            main_video_track_id,
            main_audio_track_id,
            TrackRelationship::Locked,
            &timeline_ref,
        )?;
    }
    */

    // Set output file path
    let output_path = output_dir.join("advanced_multi_track_output.mp4");

    // Create render config
    let render_config = RenderConfig::new(output_path.clone())
        .with_resolution(1280, 720)
        .with_frame_rate(30.0)
        .with_video_settings(VideoCodec::H264, 85)
        .with_audio_settings(AudioCodec::AAC, 80)
        .with_format(OutputFormat::MP4);

    println!("Rendering project to: {}", output_path.display());

    // Render the project
    let result = project.render_with_config(render_config)?;

    println!("Rendering completed successfully!");
    println!("Duration: {} seconds", result.duration.as_seconds());
    println!("Output file: {}", result.output_path.display());

    Ok(())
}
