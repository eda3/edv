/// Multi-track Editing Sample Project with Keyframe Animations
///
/// This sample demonstrates creating a project with multiple video and audio tracks,
/// and using keyframe animations to change opacity and position over time.
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
/// TMPDIR=$(pwd)/output/temp cargo run --example multi_track_keyframe_demo
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
    let mut project = Project::new("Keyframe Multi-track Sample");

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

    // Add asset
    let asset_id = project.add_asset(input_file.clone(), asset_metadata.clone());

    // Add the same file as a second asset (in real projects, different files would typically be used)
    asset_metadata
        .extra
        .insert("purpose".to_string(), "overlay".to_string());
    let overlay_asset_id = project.add_asset(input_file.clone(), asset_metadata);

    // Add tracks to timeline
    let main_video_track_id = project.timeline.add_track(TrackKind::Video);
    let overlay_video_track_id = project.timeline.add_track(TrackKind::Video);
    let audio_track_id = project.timeline.add_track(TrackKind::Audio);

    // Set track names
    if let Some(track) = project.timeline.get_track_mut(main_video_track_id) {
        track.set_name("Main Video");
    }

    if let Some(track) = project.timeline.get_track_mut(overlay_video_track_id) {
        track.set_name("Overlay with Keyframe Animation");
    }

    if let Some(track) = project.timeline.get_track_mut(audio_track_id) {
        track.set_name("Audio");
    }

    // Create clip and add to main video track
    let main_clip = Clip::new(
        ClipId::new(),
        asset_id,
        TimePosition::from_seconds(0.0),
        Duration::from_seconds(main_video_duration),
        TimePosition::from_seconds(0.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project.timeline.add_clip(main_video_track_id, main_clip)?;

    // Create overlay video clip (display for 5 seconds)
    let overlay_clip = Clip::new(
        ClipId::new(),
        overlay_asset_id,
        TimePosition::from_seconds(1.0), // Start after 1 second
        Duration::from_seconds(5.0),     // Display for 5 seconds
        TimePosition::from_seconds(0.0), // From the beginning of the original video
        TimePosition::from_seconds(5.0), // To 5 seconds in the original video
    );

    project
        .timeline
        .add_clip(overlay_video_track_id, overlay_clip)?;

    // Add audio clip (using the audio from the original video)
    let audio_clip = Clip::new(
        ClipId::new(),
        asset_id, // Use audio from the same asset
        TimePosition::from_seconds(0.0),
        Duration::from_seconds(main_video_duration),
        TimePosition::from_seconds(0.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project.timeline.add_clip(audio_track_id, audio_clip)?;

    // Add keyframe animations to the overlay

    // Opacity keyframe animation
    // Start: 0% (invisible) -> After 1 second: 100% (fully visible) -> After 4 seconds: 100% -> After 5 seconds: 0% (fade out)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(1.0), // Starting point
        0.0,                             // Starting opacity (0%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(2.0), // After 1 second
        1.0,                             // Opacity 100%
        EasingFunction::EaseOut,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(5.0), // After 4 seconds
        1.0,                             // Opacity 100%
        EasingFunction::EaseIn,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(6.0), // After 5 seconds
        0.0,                             // Opacity 0%
        EasingFunction::Linear,
    )?;

    // Scale keyframe animation (size change)
    // Start: 0.5 (50% size) -> After 5 seconds: 1.0 (100% size)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "scale",
        TimePosition::from_seconds(1.0), // Starting point
        0.5,                             // Starting scale (50%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "scale",
        TimePosition::from_seconds(6.0), // After 5 seconds
        1.0,                             // Scale 100%
        EasingFunction::EaseOut,
    )?;

    // Position keyframe animation (X-coordinate change)
    // Start: -0.3 (left-leaning) -> After 5 seconds: 0.3 (right-leaning)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "position_x",
        TimePosition::from_seconds(1.0), // Starting point
        -0.3,                            // Starting X position (left-leaning)
        EasingFunction::EaseInOut,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "position_x",
        TimePosition::from_seconds(6.0), // After 5 seconds
        0.3,                             // X position (right-leaning)
        EasingFunction::EaseInOut,
    )?;

    // Set output file path
    let output_path = output_dir.join("keyframe_animation_output.mp4");

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
