/// Multi-track Editing Sample Project
///
/// This sample demonstrates creating a project with multiple video and audio tracks,
/// and compositing them to output a final video.
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
/// TMPDIR=$(pwd)/output/temp cargo run --example multi_track_demo
/// ```
///
/// # TODO
///
/// * Fix temporary directory creation issues in WSL environment
/// * Implement more flexible media file path specification
use std::path::{Path, PathBuf};

use edv::ffmpeg::FFmpeg;
use edv::project::rendering::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
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
    let mut project = Project::new("Multi-track Sample");

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
        track.set_name("Overlay Video");
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

    // Create overlay video clip (cut part of the original video and display at a different time)
    let overlay_clip = Clip::new(
        ClipId::new(),
        overlay_asset_id,
        TimePosition::from_seconds(2.0), // Start after 2 seconds
        Duration::from_seconds(3.0),     // Display for 3 seconds
        TimePosition::from_seconds(1.0), // From 1 second in the original video
        TimePosition::from_seconds(4.0), // To 4 second in the original video
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

    // Set multi-track relationships (overlay shown as semi-transparent)
    // Note: Current API doesn't have a set_track_opacity method, so we need to use keyframes instead
    // This part is commented out
    /*
    let multi_track_manager = project.timeline.multi_track_manager_mut();
    multi_track_manager.set_track_opacity(overlay_video_track_id, 0.5)?;
    */

    // Use keyframes to set opacity instead
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(0.0),
        0.5, // 50% opacity
        edv::project::timeline::keyframes::EasingFunction::Linear,
    )?;

    // Ensure output path exists
    let output_path = output_dir.join("multi_track_output.mp4");

    // Create render config
    let render_config = RenderConfig::new(output_path.clone())
        .with_resolution(1280, 720)
        .with_frame_rate(30.0)
        .with_video_settings(VideoCodec::H264, 85)
        .with_audio_settings(AudioCodec::AAC, 80)
        .with_format(OutputFormat::MP4);

    // Start rendering
    println!("Rendering project to: {}", output_path.display());
    let result = project.render_with_config(render_config)?;

    println!("Successfully rendered to: {}", result.output_path.display());
    println!("Duration: {} seconds", result.duration.as_seconds());
    Ok(())
}
