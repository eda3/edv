/// Track composition for timeline rendering.
///
/// This module provides functionality for compositing multiple tracks
/// together for rendering, handling different track types and their
/// relationships.
///
/// ## Module Architecture
///
/// ```mermaid
/// classDiagram
///     class TrackCompositor {
///         -timeline: Timeline
///         -assets: Vec~AssetReference~
///         -intermediate_files: Vec~IntermediateFile~
///         -progress: Option~SharedProgressTracker~
///         +new(Timeline, Vec~AssetReference~): TrackCompositor
///         +set_progress_tracker(SharedProgressTracker): void
///         +compose(RenderConfig): Result
///         -prepare_tracks(RenderConfig): Result~HashMap~
///         -composite_tracks(HashMap, RenderConfig, Path): Result
///         -calculate_timeline_duration(): Duration
///         -prepare_video_track(Track, RenderConfig): Result~PreparedTrack~
///         -prepare_audio_track(Track, RenderConfig): Result~PreparedTrack~
///         -prepare_subtitle_track(Track, RenderConfig): Result~PreparedTrack~
///     }
///
///     class PreparedTrack {
///         -id: TrackId
///         -kind: TrackKind
///         -file: Option~IntermediateFile~
///         -clips: Vec~Clip~
///         -duration: Duration
///     }
///
///     class IntermediateFile {
///         -path: PathBuf
///         -_temp_dir: TempDir
///         +new(suffix: String): Result~IntermediateFile~
///         +path(): &Path
///     }
///
///     class CompositionError {
///         <<enumeration>>
///         MissingAsset(AssetId)
///         IncompatibleTracks(String)
///         AssetFileError(String)
///         IntermediateFileError(std::io::Error)
///         FFmpeg(crate::ffmpeg::Error)
///     }
///
///     TrackCompositor --> PreparedTrack: creates
///     TrackCompositor --> IntermediateFile: manages
///     TrackCompositor ..> CompositionError: produces
///     PreparedTrack --> IntermediateFile: contains
/// ```
///
/// ## Composition Process
///
/// ```mermaid
/// flowchart TD
///     Start([Start]) --> Init[Initialize TrackCompositor]
///     Init --> SetProgress[Set Progress Tracker]
///     SetProgress --> Compose[Call compose()]
///     Compose --> PrepareTracks[Prepare tracks by kind]
///     PrepareTracks --> Video[Prepare Video Tracks]
///     PrepareTracks --> Audio[Prepare Audio Tracks]
///     PrepareTracks --> Subtitle[Prepare Subtitle Tracks]
///     Video --> CollectPrepared[Collect Prepared Tracks]
///     Audio --> CollectPrepared
///     Subtitle --> CollectPrepared
///     CollectPrepared --> Composite[Composite Tracks Together]
///     Composite --> CreateFFmpeg[Create FFmpeg Command]
///     CreateFFmpeg --> AddInputs[Add Intermediate Files as Inputs]
///     AddInputs --> SetFilters[Set FFmpeg Filters]
///     SetFilters --> Execute[Execute FFmpeg Command]
///     Execute --> Cleanup[Cleanup Temporary Files]
///     Cleanup --> End([End])
///     
///     Cancel{Cancelled?} -.-> |Yes| Cleanup
///     PrepareTracks -.-> Cancel
///     Composite -.-> Cancel
/// ```
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::project::AssetId;
use crate::project::AssetReference;
use crate::project::rendering::config::{AudioCodec, RenderConfig, VideoCodec};
use crate::project::rendering::progress::{RenderStage, SharedProgressTracker};
use crate::project::timeline::keyframes::{EasingFunction, KeyframeAnimation};
use crate::project::timeline::multi_track;
use crate::project::timeline::{Clip, Timeline, Track, TrackId, TrackKind};
use crate::utility::time::{Duration, TimePosition};

/// Types of blend modes.
/// Specifies how video tracks are composited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Normal overlay (standard alpha blending)
    Normal,
    /// Additive blending (brightness is added)
    Add,
    /// Multiply blending (darker pixels are emphasized)
    Multiply,
    /// Screen blending (brighter pixels are emphasized)
    Screen,
    /// Overlay blending (contrast is emphasized)
    Overlay,
    /// Soft light blending (soft light effect)
    SoftLight,
    /// Hard light blending (strong light effect)
    HardLight,
    /// Color dodge blending (brighter areas are emphasized)
    ColorDodge,
    /// Color burn blending (darker areas are emphasized)
    ColorBurn,
    /// Difference blending (color differences are emphasized)
    Difference,
    /// Exclusion blending (soft version of difference)
    Exclusion,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for BlendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Add => write!(f, "add"),
            Self::Multiply => write!(f, "multiply"),
            Self::Screen => write!(f, "screen"),
            Self::Overlay => write!(f, "overlay"),
            Self::SoftLight => write!(f, "softlight"),
            Self::HardLight => write!(f, "hardlight"),
            Self::ColorDodge => write!(f, "colordodge"),
            Self::ColorBurn => write!(f, "colorburn"),
            Self::Difference => write!(f, "difference"),
            Self::Exclusion => write!(f, "exclusion"),
        }
    }
}

/// Simple FFmpeg command builder for composition
#[derive(Debug)]
struct FFmpegCommand {
    /// Input files for the command
    inputs: Vec<std::path::PathBuf>,
    /// Output path
    output: Option<std::path::PathBuf>,
    /// Additional arguments for the command
    args: Vec<String>,
}

impl FFmpegCommand {
    /// Creates a new FFmpeg command
    fn new() -> Self {
        Self {
            inputs: Vec::new(),
            output: None,
            args: Vec::new(),
        }
    }

    /// Adds an input file to the command
    fn add_input(&mut self, path: &Path) -> &mut Self {
        self.inputs.push(path.to_path_buf());
        self
    }

    /// Sets the output file for the command
    fn set_output(&mut self, path: &Path) -> &mut Self {
        self.output = Some(path.to_path_buf());
        self
    }

    /// Adds a complex filter to the command
    fn add_complex_filter(&mut self, filter: &str) -> &mut Self {
        self.args.push("-filter_complex".to_string());
        self.args.push(filter.to_string());
        self
    }

    /// Adds an argument with a value to the command
    fn add_arg(&mut self, arg: &str, value: &str) -> &mut Self {
        self.args.push(arg.to_string());
        self.args.push(value.to_string());
        self
    }

    /// Sets the video codec for the command
    fn set_video_codec(&mut self, codec: &str) -> &mut Self {
        self.add_arg("-c:v", codec)
    }

    /// Sets the audio codec for the command
    fn set_audio_codec(&mut self, codec: &str) -> &mut Self {
        self.add_arg("-c:a", codec)
    }

    /// Sets the frame rate for the command
    fn set_frame_rate(&mut self, fps: f64) -> &mut Self {
        self.add_arg("-r", &fps.to_string())
    }

    /// Sets the video size for the command
    fn set_video_size(&mut self, width: u32, height: u32) -> &mut Self {
        self.add_arg("-s", &format!("{}x{}", width, height))
    }

    /// Runs the FFmpeg command
    fn run(&self) -> Result<()> {
        // Create the command
        let mut command = Command::new("ffmpeg");

        // Add the inputs
        for input in &self.inputs {
            command.arg("-i").arg(input);
        }

        // Add the additional arguments
        for arg in &self.args {
            command.arg(arg);
        }

        // Add the output
        if let Some(output) = &self.output {
            command.arg(output);
        }

        // Add overwrite flag
        command.arg("-y");

        // Set up stdout and stderr
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Run the command
        let output = command
            .output()
            .map_err(|e| CompositionError::FFmpeg(crate::ffmpeg::Error::IoError(e)))?;

        // Check the exit status
        if !output.status.success() {
            // Get the error output
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CompositionError::FFmpeg(
                crate::ffmpeg::Error::ProcessTerminated {
                    exit_code: output.status.code(),
                    message: format!("FFmpeg command failed: {}", stderr),
                },
            ));
        }

        Ok(())
    }
}

/// Error types for composition operations.
#[derive(Debug, thiserror::Error)]
pub enum CompositionError {
    /// Missing asset for a clip.
    #[error("Missing asset: {0}")]
    MissingAsset(AssetId),

    /// Incompatible track kinds in composition.
    #[error("Incompatible tracks: {0}")]
    IncompatibleTracks(String),

    /// Error accessing asset files.
    #[error("Asset file error: {0}")]
    AssetFileError(String),

    /// Error creating intermediate files.
    #[error("Intermediate file error: {0}")]
    IntermediateFileError(#[from] std::io::Error),

    /// FFmpeg error during composition.
    #[error("FFmpeg error: {0}")]
    FFmpeg(#[from] crate::ffmpeg::Error),
}

/// Type alias for composition operation results.
pub type Result<T> = std::result::Result<T, CompositionError>;

/// Intermediate file created during the composition process.
#[derive(Debug)]
struct IntermediateFile {
    /// Path to the intermediate file.
    path: std::path::PathBuf,

    /// Temporary directory containing the file.
    _temp_dir: tempfile::TempDir,
}

impl IntermediateFile {
    /// Creates a new intermediate file with the given suffix.
    fn new(suffix: &str) -> Result<Self> {
        let temp_dir = tempfile::TempDir::new()?;
        let file_name = format!("intermediate_{}.{}", uuid::Uuid::new_v4(), suffix);
        let path = temp_dir.path().join(file_name);

        Ok(Self {
            path,
            _temp_dir: temp_dir,
        })
    }

    /// Gets the path to the intermediate file.
    fn path(&self) -> &Path {
        &self.path
    }
}

/// Represents a track prepared for composition.
#[derive(Debug)]
struct PreparedTrack {
    /// ID of the original track.
    id: TrackId,

    /// Kind of track (video, audio, etc.).
    kind: TrackKind,

    /// Intermediate file for the rendered track.
    file: Option<IntermediateFile>,

    /// Clips in the track.
    clips: Vec<Clip>,

    /// Duration of the track.
    duration: Duration,
}

/// Manages the composition of multiple tracks for rendering.
#[derive(Debug)]
pub struct TrackCompositor {
    /// The timeline being composited.
    timeline: Timeline,

    /// Available assets for the composition.
    assets: Vec<AssetReference>,

    /// Intermediate files created during composition.
    intermediate_files: Vec<IntermediateFile>,

    /// Progress tracker for the composition.
    progress: Option<SharedProgressTracker>,

    /// Whether to optimize for complex timelines.
    optimize_complex: bool,
}

impl TrackCompositor {
    /// Creates a new track compositor.
    ///
    /// # Arguments
    ///
    /// * `timeline` - The timeline to composite
    /// * `assets` - The available assets for the composition
    ///
    /// # Returns
    ///
    /// A new `TrackCompositor` instance.
    #[must_use]
    pub fn new(timeline: Timeline, assets: Vec<AssetReference>) -> Self {
        Self {
            timeline,
            assets,
            intermediate_files: Vec::new(),
            progress: None,
            optimize_complex: false,
        }
    }

    /// Sets the progress tracker for the composition.
    pub fn set_progress_tracker(&mut self, progress: SharedProgressTracker) {
        self.progress = Some(progress);
    }

    /// Sets whether to optimize for complex timelines.
    pub fn set_optimize_complex(&mut self, optimize: bool) {
        self.optimize_complex = optimize;
    }

    /// Gets the asset with the given ID.
    fn get_asset(&self, asset_id: AssetId) -> Option<&AssetReference> {
        self.assets.iter().find(|asset| asset.id == asset_id)
    }

    /// Updates the composition progress.
    fn update_progress(&self, stage: RenderStage) {
        if let Some(progress) = &self.progress {
            progress.set_stage(stage);
        }
    }

    /// Checks if the composition has been cancelled.
    fn is_cancelled(&self) -> bool {
        self.progress.as_ref().map_or(false, |p| p.is_cancelled())
    }

    /// Prepares tracks for composition.
    ///
    /// # Arguments
    ///
    /// * `config` - The rendering configuration
    ///
    /// # Returns
    ///
    /// A map of track IDs to prepared tracks, or an error if preparation failed.
    fn prepare_tracks(&mut self, config: &RenderConfig) -> Result<HashMap<TrackId, PreparedTrack>> {
        self.update_progress(RenderStage::Preparing);

        let mut prepared_tracks = HashMap::new();
        let _timeline_duration = self.calculate_timeline_duration();

        // 先にすべてのトラックとその種類を収集
        let tracks_to_process: Vec<(TrackId, TrackKind, Vec<Clip>)> = self
            .timeline
            .get_tracks()
            .iter()
            .filter(|track| !track.get_clips().is_empty())
            .map(|track| (track.id(), track.kind(), track.get_clips().to_vec()))
            .collect();

        // 処理能力に基づいて並列処理を最適化（複雑なタイムラインの場合）
        let is_complex_timeline = self.optimize_complex && tracks_to_process.len() > 4;

        if is_complex_timeline {
            // 利用可能なCPUコア数に基づいてスレッド数を決定
            let num_cpus = num_cpus::get();
            let thread_count = (num_cpus / 2).max(2).min(tracks_to_process.len());

            println!(
                "🚀 パフォーマンス最適化: {}トラックを{}スレッドで並列処理します",
                tracks_to_process.len(),
                thread_count
            );

            use rayon::prelude::*;
            use std::sync::Mutex;

            // 結果を保存するためのスレッドセーフなコンテナ
            let prepared_results = Mutex::new(Vec::with_capacity(tracks_to_process.len()));
            let temp_files = Mutex::new(Vec::new());

            // スレッドプールを構成して並列処理を実行
            rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .unwrap()
                .install(|| {
                    tracks_to_process
                        .par_iter()
                        .for_each(|(track_id, kind, clips)| {
                            // キャンセルされた場合はスキップ
                            if self.is_cancelled() {
                                return;
                            }

                            let result = match kind {
                                TrackKind::Video => {
                                    match self
                                        .prepare_video_track_parallel(*track_id, clips, config)
                                    {
                                        Ok((prepared_track, file)) => {
                                            // 中間ファイルを安全に保存
                                            if let Some(f) = file {
                                                let mut files = temp_files.lock().unwrap();
                                                files.push(f);
                                            }
                                            Ok((*track_id, prepared_track))
                                        }
                                        Err(e) => Err(e),
                                    }
                                }
                                TrackKind::Audio => {
                                    match self
                                        .prepare_audio_track_parallel(*track_id, clips, config)
                                    {
                                        Ok((prepared_track, file)) => {
                                            // 中間ファイルを安全に保存
                                            if let Some(f) = file {
                                                let mut files = temp_files.lock().unwrap();
                                                files.push(f);
                                            }
                                            Ok((*track_id, prepared_track))
                                        }
                                        Err(e) => Err(e),
                                    }
                                }
                                TrackKind::Subtitle => {
                                    match self
                                        .prepare_subtitle_track_parallel(*track_id, clips, config)
                                    {
                                        Ok(prepared_track) => Ok((*track_id, prepared_track)),
                                        Err(e) => Err(e),
                                    }
                                }
                            };

                            // 結果を安全に保存
                            let mut results = prepared_results.lock().unwrap();
                            match result {
                                Ok((id, track)) => results.push(Ok((id, track))),
                                Err(e) => results.push(Err(e)),
                            }
                        });
                });

            // 中間ファイルを保存
            let files = temp_files.into_inner().unwrap();
            for file in files {
                self.intermediate_files.push(file);
            }

            // 結果を処理
            let results = prepared_results.into_inner().unwrap();
            for result in results {
                match result {
                    Ok((id, track)) => {
                        prepared_tracks.insert(id, track);
                    }
                    Err(e) => return Err(e),
                }
            }

            // 各トラックにファイルを関連付け
            // tracks_to_processの順番で処理されたことを利用
            for track_id in tracks_to_process.iter().map(|(id, _, _)| id) {
                if let Some(track) = prepared_tracks.get_mut(track_id) {
                    if !self.intermediate_files.is_empty() {
                        track.file = Some(self.intermediate_files.remove(0));
                    }
                }
            }
        } else {
            // 通常の逐次処理（複雑でないタイムラインの場合）
            for (track_id, kind, clips) in tracks_to_process {
                let prepared_track = match kind {
                    TrackKind::Video => {
                        self.prepare_video_track_from_data(track_id, &clips, config)?
                    }
                    TrackKind::Audio => {
                        self.prepare_audio_track_from_data(track_id, &clips, config)?
                    }
                    TrackKind::Subtitle => {
                        self.prepare_subtitle_track_from_data(track_id, &clips, config)?
                    }
                };

                prepared_tracks.insert(track_id, prepared_track);

                // キャンセルされたかチェック
                if self.is_cancelled() {
                    return Err(CompositionError::IncompatibleTracks(
                        "Composition cancelled".to_string(),
                    ));
                }
            }
        }

        Ok(prepared_tracks)
    }

    /// Prepares a video track from clip data.
    ///
    /// # Arguments
    ///
    /// * `track_id` - ID of the track to prepare
    /// * `clips` - Clips in the track
    /// * `config` - Rendering configuration
    ///
    /// # Returns
    ///
    /// The prepared track, or an error if preparation failed.
    fn prepare_video_track_from_data(
        &mut self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // Create a temporary file for the rendered track
        let intermediate_file = IntermediateFile::new("mp4")?;
        self.intermediate_files.push(intermediate_file);

        // For simplicity, we'll just return a prepared track
        // In a real implementation, this would process the clips and create the intermediate file
        let duration = clips
            .iter()
            .map(|clip| clip.position() + clip.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| TimePosition::from_seconds(0.0))
            .to_duration();

        Ok(PreparedTrack {
            id: track_id,
            kind: TrackKind::Video,
            file: Some(self.intermediate_files.pop().unwrap()),
            clips: clips.to_vec(),
            duration,
        })
    }

    /// Prepares an audio track from clip data.
    ///
    /// # Arguments
    ///
    /// * `track_id` - ID of the track to prepare
    /// * `clips` - Clips in the track
    /// * `config` - Rendering configuration
    ///
    /// # Returns
    ///
    /// The prepared track, or an error if preparation failed.
    fn prepare_audio_track_from_data(
        &mut self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // Create a temporary file for the rendered track
        let intermediate_file = IntermediateFile::new("aac")?;
        self.intermediate_files.push(intermediate_file);

        // For simplicity, we'll just return a prepared track
        // In a real implementation, this would process the clips and create the intermediate file
        let duration = clips
            .iter()
            .map(|clip| clip.position() + clip.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| TimePosition::from_seconds(0.0))
            .to_duration();

        Ok(PreparedTrack {
            id: track_id,
            kind: TrackKind::Audio,
            file: Some(self.intermediate_files.pop().unwrap()),
            clips: clips.to_vec(),
            duration,
        })
    }

    /// Prepares a subtitle track from clip data.
    ///
    /// # Arguments
    ///
    /// * `track_id` - ID of the track to prepare
    /// * `clips` - Clips in the track
    /// * `config` - Rendering configuration
    ///
    /// # Returns
    ///
    /// The prepared track, or an error if preparation failed.
    fn prepare_subtitle_track_from_data(
        &mut self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // For simplicity, we'll just return a prepared track without a file
        // In a real implementation, this would generate a subtitle file
        let duration = clips
            .iter()
            .map(|clip| clip.position() + clip.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| TimePosition::from_seconds(0.0))
            .to_duration();

        Ok(PreparedTrack {
            id: track_id,
            kind: TrackKind::Subtitle,
            file: None, // No intermediate file for subtitles
            clips: clips.to_vec(),
            duration,
        })
    }

    /// Calculates the duration of the timeline.
    ///
    /// # Returns
    ///
    /// The duration of the timeline in seconds.
    fn calculate_timeline_duration(&self) -> Duration {
        // Find the max end time of all clips across all tracks
        let mut max_duration = Duration::zero();

        for track in self.timeline.get_tracks() {
            for clip in track.get_clips() {
                let clip_end = clip.position() + clip.duration();
                if clip_end.as_seconds() > max_duration.as_seconds() {
                    max_duration = Duration::from_seconds(clip_end.as_seconds());
                }
            }
        }

        max_duration
    }

    /// Composes the timeline into a video file.
    ///
    /// # Arguments
    ///
    /// * `config` - The rendering configuration
    ///
    /// # Returns
    ///
    /// `Ok(())` if composition was successful, or an error if composition failed.
    pub fn compose(&mut self, config: &RenderConfig) -> Result<()> {
        // Ensure the timeline has tracks
        if self.timeline.get_tracks().is_empty() {
            return Err(CompositionError::IncompatibleTracks(
                "Timeline has no tracks".to_string(),
            ));
        }

        // Prepare tracks for composition
        let prepared_tracks = self.prepare_tracks(config)?;

        // Check if the composition has been canceled
        if self.is_cancelled() {
            return Err(CompositionError::IncompatibleTracks(
                "Composition cancelled".to_string(),
            ));
        }

        // Composite the prepared tracks
        self.composite_tracks(prepared_tracks, config, &config.output_path)?;

        Ok(())
    }

    /// Generates an FFmpeg filter graph for multi-track video composition.
    ///
    /// This function creates a complex filtergraph to layer multiple video tracks
    /// according to their z-order and transparency.
    ///
    /// # Arguments
    ///
    /// * `video_tracks` - The prepared video tracks to compose
    /// * `config` - The render configuration
    ///
    /// # Returns
    ///
    /// A string containing the FFmpeg filtergraph definition
    fn generate_video_filtergraph(
        &self,
        video_tracks: &[&PreparedTrack],
        config: &RenderConfig,
    ) -> String {
        if video_tracks.is_empty() {
            return String::new();
        }

        // Filter parts and overlay chain variables
        let mut filter_parts = Vec::new();
        let mut overlay_chain = String::new();

        // Determine Z-order considering multi-track relationships
        let mut ordered_tracks: Vec<&PreparedTrack> = Vec::new();

        // Get sort order from track relationship manager
        let _multi_track_manager = self.timeline.multi_track_manager();

        // First filter only visible tracks
        let filtered_tracks = video_tracks
            .iter()
            .filter(|_track| {
                // In a real implementation, we'd check if the track is visible
                true
            })
            .copied()
            .collect::<Vec<_>>();

        // Sort tracks based on relationships
        // Process from lower layer to upper layer (z-index from low to high)
        if !filtered_tracks.is_empty() {
            // Track unprocessed tracks
            let mut remaining_tracks: Vec<&PreparedTrack> = filtered_tracks.clone();

            // Order based on track relationships (for simple cases)
            // In implementation, use track relationship manager for proper ordering

            // Simple alphabetical sort by track ID (replace in actual implementation)
            remaining_tracks.sort_by(|a, b| format!("{:?}", a.id).cmp(&format!("{:?}", b.id)));

            ordered_tracks = remaining_tracks;
        }

        // Process each track to build filter graph
        for (i, track) in ordered_tracks.iter().enumerate() {
            let input_index = i; // Input index matches FFmpeg input order

            // Ensure alpha channel support (for transparent compositing)
            let format_filter = format!(
                "[{input_index}:v] format=yuva420p",
                input_index = input_index
            );

            // Scale to output size
            let scale_filter = format!(
                "{} ,scale={width}:{height},setsar=1",
                format_filter,
                width = config.width,
                height = config.height
            );

            // Add keyframe and animation support
            let mut track_filters = scale_filter;

            // Check and apply track keyframes
            if let Some(track_obj) = self.timeline.get_track(track.id) {
                if let Some(keyframes) = track_obj.keyframes() {
                    // Process opacity keyframes
                    if keyframes.has_property("opacity") {
                        // Get opacity at specific timeline position
                        // (In current implementation, simplified to get at middle position)
                        let opacity_pos =
                            TimePosition::from_seconds(track.duration.as_seconds() / 2.0);
                        let opacity_value = keyframes
                            .get_value_at("opacity", opacity_pos)
                            .unwrap_or(1.0); // Default to fully opaque

                        track_filters =
                            format!("{},colorchannelmixer=aa={}", track_filters, opacity_value);
                    }

                    // Process other keyframe animations
                    // Position, rotation, scale, etc.
                    if keyframes.has_property("position_x") && keyframes.has_property("position_y")
                    {
                        // Apply position keyframes
                        let pos_x = keyframes
                            .get_value_at("position_x", TimePosition::from_seconds(0.0))
                            .unwrap_or(0.0);
                        let pos_y = keyframes
                            .get_value_at("position_y", TimePosition::from_seconds(0.0))
                            .unwrap_or(0.0);

                        track_filters = format!(
                            "{},pad=iw:ih:{}:{}:color=black@0",
                            track_filters, pos_x, pos_y
                        );
                    }

                    if keyframes.has_property("scale") {
                        // Apply scale keyframes
                        let scale_value = keyframes
                            .get_value_at("scale", TimePosition::from_seconds(0.0))
                            .unwrap_or(1.0);

                        track_filters = format!(
                            "{},scale=iw*{}:ih*{}",
                            track_filters, scale_value, scale_value
                        );
                    }
                }
            }

            // Add final track output label
            track_filters = format!("{} [v{}]", track_filters, i);
            filter_parts.push(track_filters);

            // Build composition chain
            if i == 0 {
                // First track is used as base layer
                overlay_chain = format!("[v0]");
            } else {
                // Get blend mode from track relationship
                let blend_mode = self.get_blend_mode_for_track(track.id);

                // Use appropriate overlay filter based on selected blend mode
                let overlay_filter = match blend_mode {
                    BlendMode::Normal => format!(
                        "{prev}[v{current}] overlay=shortest=1:format=yuv420",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::Add => format!(
                        "{prev}[v{current}] blend=all_mode=addition:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::Multiply => format!(
                        "{prev}[v{current}] blend=all_mode=multiply:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::Screen => format!(
                        "{prev}[v{current}] blend=all_mode=screen:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::Overlay => format!(
                        "{prev}[v{current}] blend=all_mode=overlay:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::SoftLight => format!(
                        "{prev}[v{current}] blend=all_mode=softlight:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::HardLight => format!(
                        "{prev}[v{current}] blend=all_mode=hardlight:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::ColorDodge => format!(
                        "{prev}[v{current}] blend=all_mode=colordodge:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::ColorBurn => format!(
                        "{prev}[v{current}] blend=all_mode=colorburn:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::Difference => format!(
                        "{prev}[v{current}] blend=all_mode=difference:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                    BlendMode::Exclusion => format!(
                        "{prev}[v{current}] blend=all_mode=exclusion:all_opacity=1",
                        prev = overlay_chain,
                        current = i
                    ),
                };

                let next_output = format!("[v{next}]", next = i + 1);
                filter_parts.push(format!("{} {}", overlay_filter, next_output));
                overlay_chain = next_output;
            }
        }

        // All filter parts are joined with semicolon
        filter_parts.join(";")
    }

    /// Gets the blend mode for a specific track.
    ///
    /// In a real implementation, this would retrieve the blend mode from track properties.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track
    ///
    /// # Returns
    ///
    /// The blend mode for the track
    fn get_blend_mode_for_track(&self, track_id: TrackId) -> BlendMode {
        // Get blend mode from track
        if let Some(track) = self.timeline.get_track(track_id) {
            // First check blend mode from keyframes
            if let Some(keyframes) = track.keyframes() {
                if keyframes.has_property("blend_mode") {
                    // Blend mode is stored as a number
                    // 0: Normal, 1: Add, 2: Multiply, 3: Screen
                    if let Some(blend_value) =
                        keyframes.get_value_at("blend_mode", TimePosition::from_seconds(0.0))
                    {
                        let blend_index = blend_value.round() as i32;
                        return match blend_index {
                            0 => BlendMode::Normal,
                            1 => BlendMode::Add,
                            2 => BlendMode::Multiply,
                            3 => BlendMode::Screen,
                            4 => BlendMode::Overlay,
                            5 => BlendMode::SoftLight,
                            6 => BlendMode::HardLight,
                            7 => BlendMode::ColorDodge,
                            8 => BlendMode::ColorBurn,
                            9 => BlendMode::Difference,
                            10 => BlendMode::Exclusion,
                            _ => BlendMode::Normal, // Default
                        };
                    }
                }
            }

            // Default blend mode based on track type
            // Example: Use screen mode for overlay tracks
            let track_name = track.name().to_lowercase();
            if track_name.contains("overlay") {
                return BlendMode::Screen;
            } else if track_name.contains("effect") {
                return BlendMode::Add;
            } else if track_name.contains("shadow") {
                return BlendMode::Multiply;
            }
        }

        // Default is Normal
        BlendMode::Normal
    }

    /// Generates an FFmpeg filter graph for multi-track audio composition.
    ///
    /// This function creates filters to mix multiple audio tracks together with
    /// proper volume levels.
    ///
    /// # Arguments
    ///
    /// * `audio_tracks` - The prepared audio tracks to compose
    /// * `config` - The render configuration
    ///
    /// # Returns
    ///
    /// A string containing the FFmpeg audio filtergraph definition
    fn generate_audio_filtergraph(
        &self,
        audio_tracks: &[&PreparedTrack],
        _config: &RenderConfig,
    ) -> String {
        if audio_tracks.is_empty() {
            return String::new();
        }

        // Filter parts and mixing input variables
        let mut filter_parts = Vec::new();
        let mut amix_inputs = Vec::new();

        // Consider multi-track relationships and priority
        let _multi_track_manager = self.timeline.multi_track_manager();

        // Process each track (order is not important - all mixed)
        for (i, track) in audio_tracks.iter().enumerate() {
            let input_index = i; // Input index matches FFmpeg input order

            // Apply volume adjustment
            let volume_value = self.get_track_volume(track.id);

            // Audio normalization and format setting
            let audio_filter = format!(
                "[{input_index}:a] aformat=sample_fmts=fltp:channel_layouts=stereo,volume={volume}",
                input_index = input_index,
                volume = volume_value
            );

            // Time line processing and effect
            let mut processed_filter = audio_filter;

            // Apply keyframe animations
            if let Some(track_obj) = self.timeline.get_track(track.id) {
                if let Some(keyframes) = track_obj.keyframes() {
                    // Process volume keyframes
                    if keyframes.has_property("volume") {
                        // Time line volume value at specific position
                        // (In actual implementation, check multiple points for keyframes)

                        // Fade in/out implementation
                        let duration = track.duration.as_seconds();

                        // Fade in (first 1 second)
                        if duration > 1.0 {
                            processed_filter = format!("{},afade=t=in:st=0:d=1", processed_filter);
                        }

                        // Fade out (last 1 second)
                        if duration > 2.0 {
                            processed_filter = format!(
                                "{},afade=t=out:st={}:d=1",
                                processed_filter,
                                duration - 1.0
                            );
                        }
                    }

                    // EQ setting keyframes (bass/treble adjustment, etc.)
                    if keyframes.has_property("bass") || keyframes.has_property("treble") {
                        let bass = keyframes
                            .get_value_at("bass", TimePosition::from_seconds(0.0))
                            .unwrap_or(0.0);
                        let treble = keyframes
                            .get_value_at("treble", TimePosition::from_seconds(0.0))
                            .unwrap_or(0.0);

                        // Simple EQ adjustment (actual implementation is more detailed)
                        if bass != 0.0 || treble != 0.0 {
                            processed_filter = format!(
                                "{},equalizer=f=100:t=h:width=200:g={}:f=10000:t=h:width=2000:g={}",
                                processed_filter, bass, treble
                            );
                        }
                    }
                }
            }

            // Process considering track relationships
            // Example: Mute setting and solo track processing
            let mut is_muted = false;

            // Check mute state (actual implementation gets from track properties)
            if let Some(track_obj) = self.timeline.get_track(track.id) {
                if let Some(keyframes) = track_obj.keyframes() {
                    if let Some(mute_value) =
                        keyframes.get_value_at("mute", TimePosition::from_seconds(0.0))
                    {
                        is_muted = mute_value > 0.5; // Mute if 0.5 or more
                    }
                }
            }

            // Mute track case is set volume to 0
            if is_muted {
                processed_filter = format!("{},volume=0", processed_filter);
            }

            // Add output label
            processed_filter = format!("{} [a{}]", processed_filter, i);
            filter_parts.push(processed_filter);
            amix_inputs.push(format!("[a{}]", i));
        }

        // If there are multiple tracks, add amix filter
        if audio_tracks.len() > 1 {
            // Advanced mixing parameters
            // duration=longest: Match longest track
            // normalize=0: Do not normalize volume (manual setting priority)
            // dropout_transition: Fade out time at track end
            let amix_filter = format!(
                "{} amix=inputs={}:duration=longest:normalize=0:dropout_transition=0.5 [aout]",
                amix_inputs.join(""),
                audio_tracks.len()
            );
            filter_parts.push(amix_filter);
        } else if !audio_tracks.is_empty() {
            // Single track case is direct mapping
            filter_parts.push(format!("{} asetpts=PTS-STARTPTS [aout]", amix_inputs[0]));
        }

        // All filter parts are joined with semicolon
        filter_parts.join(";")
    }

    /// Gets the volume level for a specific track.
    ///
    /// In a real implementation, this would retrieve the volume from track properties.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track
    ///
    /// # Returns
    ///
    /// The volume level for the track (1.0 is normal)
    fn get_track_volume(&self, track_id: TrackId) -> f64 {
        // Get volume setting from track
        if let Some(track) = self.timeline.get_track(track_id) {
            // Check volume value from keyframes
            if let Some(keyframes) = track.keyframes() {
                // First check mute state
                if keyframes.has_property("mute") {
                    if let Some(mute_value) =
                        keyframes.get_value_at("mute", TimePosition::from_seconds(0.0))
                    {
                        if mute_value > 0.5 {
                            return 0.0; // Volume 0 when muted
                        }
                    }
                }

                // Then check direct volume property
                if keyframes.has_property("volume") {
                    if let Some(volume) =
                        keyframes.get_value_at("volume", TimePosition::from_seconds(0.0))
                    {
                        // Check if value is in valid range (0.0~2.0)
                        if volume >= 0.0 {
                            return volume.min(2.0); // Limit to maximum 2.0 (200%)
                        }
                    }
                }
            }

            // Default volume based on track type
            let track_name = track.name().to_lowercase();
            if track_name.contains("background") {
                return 0.5; // Slightly lower volume for background
            } else if track_name.contains("effect") {
                return 0.7; // Moderate volume for effects
            } else if track_name.contains("main") {
                return 1.0; // Normal volume for main track
            }
        }

        // Default volume is 1.0 (100%)
        1.0
    }

    /// Composite the prepared tracks together.
    ///
    /// # Arguments
    ///
    /// * `prepared_tracks` - The prepared tracks to composite
    /// * `config` - The rendering configuration
    /// * `output_path` - The path to save the output file
    ///
    /// # Returns
    ///
    /// `Ok(())` if composition was successful, or an error if composition failed.
    fn composite_tracks(
        &mut self,
        prepared_tracks: HashMap<TrackId, PreparedTrack>,
        config: &RenderConfig,
        output_path: &Path,
    ) -> Result<()> {
        if prepared_tracks.is_empty() {
            return Err(CompositionError::IncompatibleTracks(
                "No prepared tracks to composite".to_string(),
            ));
        }

        // Build FFmpeg command
        let mut ffmpeg = FFmpegCommand::new();

        // Track classify by type
        let mut video_tracks: Vec<&PreparedTrack> = Vec::new();
        let mut audio_tracks: Vec<&PreparedTrack> = Vec::new();
        let mut subtitle_tracks: Vec<&PreparedTrack> = Vec::new();

        // Add all input files and classify by track type
        self.update_progress(RenderStage::Preparing);

        for track in prepared_tracks.values() {
            if let Some(file) = &track.file {
                ffmpeg.add_input(file.path());

                match track.kind {
                    TrackKind::Video => video_tracks.push(track),
                    TrackKind::Audio => audio_tracks.push(track),
                    TrackKind::Subtitle => subtitle_tracks.push(track),
                }
            }
        }

        // Build filter graph
        let mut complex_filter = String::new();

        // Build video track filter graph
        if !video_tracks.is_empty() {
            complex_filter.push_str(&self.generate_video_filtergraph(&video_tracks, config));
        }

        // Build audio track filter graph
        if !audio_tracks.is_empty() {
            // If video filter exists, separate with semicolon
            if !complex_filter.is_empty() {
                complex_filter.push(';');
            }
            complex_filter.push_str(&self.generate_audio_filtergraph(&audio_tracks, config));
        }

        // If filter exists, add filter to command
        if !complex_filter.is_empty() {
            ffmpeg.add_complex_filter(&complex_filter);

            // Filter output mapping
            if !video_tracks.is_empty() {
                ffmpeg.add_arg("-map", &format!("[v{}]", video_tracks.len()));
            }
            if !audio_tracks.is_empty() {
                ffmpeg.add_arg("-map", "[aout]");
            }
        } else {
            // If no filter, use default mapping
            if !video_tracks.is_empty() {
                ffmpeg.add_arg("-map", "0:v");
            }
            if !audio_tracks.is_empty() {
                ffmpeg.add_arg("-map", "0:a");
            }
        }

        // Subtitle track mapping
        if !subtitle_tracks.is_empty() {
            // Subtitle track mapping logic (simplified)
            ffmpeg.add_arg("-map", "0:s");
        }

        // Output format and quality setting
        ffmpeg.set_output(output_path);

        // Video codec setting
        if !video_tracks.is_empty() {
            ffmpeg.add_arg("-c:v", config.video_codec.to_ffmpeg_codec());
            ffmpeg.add_arg("-pix_fmt", "yuv420p"); // Standard pixel format for compatibility

            // Quality setting
            if config.video_codec != VideoCodec::Copy {
                // Example: CRF setting for H.264 / H.265
                if config.video_codec == VideoCodec::H264 || config.video_codec == VideoCodec::H265
                {
                    ffmpeg.add_arg("-crf", "23"); // Default quality value
                }
            }

            // Preset setting (balance encoding speed and compression rate)
            if config.video_codec == VideoCodec::H264 || config.video_codec == VideoCodec::H265 {
                ffmpeg.add_arg("-preset", "medium");
            }

            // Frame rate setting
            ffmpeg.add_arg("-r", &config.frame_rate.to_string());
        }

        // Audio codec setting
        if !audio_tracks.is_empty() {
            ffmpeg.add_arg("-c:a", config.audio_codec.to_ffmpeg_codec());
            // Audio quality setting
            if config.audio_codec != AudioCodec::Copy {
                ffmpeg.add_arg("-b:a", "192k");
            }
        }

        // Subtitle setting
        if !subtitle_tracks.is_empty() {
            ffmpeg.add_arg("-c:s", "mov_text"); // Compatible subtitle format
        }

        // Optimization for complex timeline
        if self.optimize_complex {
            // Optimization options for complex timeline
            if video_tracks.len() > 3 {
                // Use multi-pass encoding
                ffmpeg.args.push("-pass".to_string());
                ffmpeg.args.push("1".to_string());

                // Adjust video bit rate
                ffmpeg.args.push("-b:v".to_string());
                ffmpeg
                    .args
                    .push(format!("{}k", config.width * config.height / 1000));

                // Specify thread count
                ffmpeg.args.push("-threads".to_string());
                ffmpeg.args.push(num_cpus::get().to_string());
            }
        }

        // Start rendering
        self.update_progress(RenderStage::Rendering);

        // Execute FFmpeg
        ffmpeg.run()?;

        // Complete
        self.update_progress(RenderStage::PostProcessing);

        // Rendering complete
        self.update_progress(RenderStage::Complete);

        Ok(())
    }

    /// 並列処理用のビデオトラック準備メソッド
    ///
    /// # Arguments
    ///
    /// * `track_id` - トラックID
    /// * `clips` - トラック内のクリップ
    /// * `config` - レンダリング設定
    ///
    /// # Returns
    ///
    /// 準備されたトラックと中間ファイルのタプル、またはエラー
    fn prepare_video_track_parallel(
        &self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<(PreparedTrack, Option<IntermediateFile>)> {
        // 中間ファイルを作成
        let intermediate_file = IntermediateFile::new("mp4")?;

        // ここで実際のビデオ処理ロジックを実装...

        // 最長のクリップ位置+長さを計算 = トラック長さ
        let duration = clips
            .iter()
            .map(|clip| clip.position() + clip.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| TimePosition::from_seconds(0.0))
            .to_duration();

        let prepared_track = PreparedTrack {
            id: track_id,
            kind: TrackKind::Video,
            file: None, // IntermediateFileを直接渡せないので、別で返す
            clips: clips.to_vec(),
            duration,
        };

        Ok((prepared_track, Some(intermediate_file)))
    }

    /// 並列処理用のオーディオトラック準備メソッド
    ///
    /// # Arguments
    ///
    /// * `track_id` - トラックID
    /// * `clips` - トラック内のクリップ
    /// * `config` - レンダリング設定
    ///
    /// # Returns
    ///
    /// 準備されたトラックと中間ファイルのタプル、またはエラー
    fn prepare_audio_track_parallel(
        &self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<(PreparedTrack, Option<IntermediateFile>)> {
        // 中間ファイルを作成
        let intermediate_file = IntermediateFile::new("aac")?;

        // ここで実際のオーディオ処理ロジックを実装...

        // 最長のクリップ位置+長さを計算 = トラック長さ
        let duration = clips
            .iter()
            .map(|clip| clip.position() + clip.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| TimePosition::from_seconds(0.0))
            .to_duration();

        let prepared_track = PreparedTrack {
            id: track_id,
            kind: TrackKind::Audio,
            file: None, // IntermediateFileを直接渡せないので、別で返す
            clips: clips.to_vec(),
            duration,
        };

        Ok((prepared_track, Some(intermediate_file)))
    }

    /// 並列処理用の字幕トラック準備メソッド
    ///
    /// # Arguments
    ///
    /// * `track_id` - トラックID
    /// * `clips` - トラック内のクリップ
    /// * `config` - レンダリング設定
    ///
    /// # Returns
    ///
    /// 準備されたトラック、またはエラー
    fn prepare_subtitle_track_parallel(
        &self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // 字幕トラックは中間ファイルを使用しない

        // 最長のクリップ位置+長さを計算 = トラック長さ
        let duration = clips
            .iter()
            .map(|clip| clip.position() + clip.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| TimePosition::from_seconds(0.0))
            .to_duration();

        let prepared_track = PreparedTrack {
            id: track_id,
            kind: TrackKind::Subtitle,
            file: None, // 中間ファイルなし
            clips: clips.to_vec(),
            duration,
        };

        Ok(prepared_track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::timeline::keyframes::KeyframeAnimation;
    use crate::utility::time::{Duration, TimePosition};

    // Helper function to create test timeline and track
    fn create_test_timeline_with_track(
        track_name: &str,
        add_keyframes: bool,
    ) -> (Timeline, TrackId) {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        let track = timeline.get_track_mut(track_id).unwrap();
        track.set_name(track_name);

        if add_keyframes {
            // Create KeyframeAnimation with Duration
            let mut keyframes = KeyframeAnimation::new(Duration::from_seconds(10.0));

            // Create tracks for each property
            keyframes.create_track_if_missing("blend_mode").unwrap();
            keyframes.create_track_if_missing("volume").unwrap();
            keyframes.create_track_if_missing("mute").unwrap();

            // Add keyframes (with easing function)
            keyframes
                .add_keyframe(
                    "blend_mode",
                    TimePosition::from_seconds(0.0),
                    3.0,
                    EasingFunction::Linear,
                )
                .unwrap();
            keyframes
                .add_keyframe(
                    "volume",
                    TimePosition::from_seconds(0.0),
                    1.5,
                    EasingFunction::Linear,
                )
                .unwrap();
            keyframes
                .add_keyframe(
                    "mute",
                    TimePosition::from_seconds(0.0),
                    0.0,
                    EasingFunction::Linear,
                )
                .unwrap();

            // Pass as Option<KeyframeAnimation>
            track.set_keyframes(Some(keyframes));
        }

        (timeline, track_id)
    }

    #[test]
    fn test_get_blend_mode_for_track_from_keyframes() {
        let (timeline, track_id) = create_test_timeline_with_track("test_track", true);
        let mut compositor = TrackCompositor::new(timeline, vec![]);

        // Get blend mode from keyframes
        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(blend_mode, BlendMode::Screen);

        // New blend mode test
        if let Some(track) = compositor.timeline.get_track_mut(track_id) {
            if let Some(keyframes) = track.keyframes_mut() {
                // Check if keyframe property exists
                keyframes.create_track_if_missing("blend_mode").unwrap();

                // Try to remove existing keyframes at 0.0 seconds
                // If it fails with KeyframeNotFound, that's fine - we just continue
                let _ = keyframes.remove_keyframe("blend_mode", TimePosition::from_seconds(0.0));

                // Add the new keyframe
                keyframes
                    .add_keyframe(
                        "blend_mode",
                        TimePosition::from_seconds(0.0),
                        4.0, // Overlay
                        EasingFunction::Linear,
                    )
                    .unwrap();
            }
        }

        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(blend_mode, BlendMode::Overlay);
    }

    #[test]
    fn test_get_blend_mode_for_track_from_name() {
        let (timeline, track_id) = create_test_timeline_with_track("overlay_track", false);
        let compositor = TrackCompositor::new(timeline, vec![]);

        // Get blend mode from track name
        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(blend_mode, BlendMode::Screen);

        // New blend mode test
        let (timeline, track_id) = create_test_timeline_with_track("softlight_track", false);
        let compositor = TrackCompositor::new(timeline, vec![]);
        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(blend_mode, BlendMode::Normal); // Default is Normal
    }

    #[test]
    fn test_get_blend_mode_for_track_default() {
        let (timeline, track_id) = create_test_timeline_with_track("default_track", false);
        let compositor = TrackCompositor::new(timeline, vec![]);

        // Get default blend mode
        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(blend_mode, BlendMode::Normal);
    }

    #[test]
    fn test_get_track_volume_from_keyframes() {
        let (timeline, track_id) = create_test_timeline_with_track("Test Track", true);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let volume = compositor.get_track_volume(track_id);
        assert_eq!(volume, 1.5, "Should return 1.5 volume from keyframes");

        // Mute state test - Create new timeline
        let (mut timeline_muted, track_id_muted) =
            create_test_timeline_with_track("Test Track", true);
        let track = timeline_muted.get_track_mut(track_id_muted).unwrap();
        if let Some(keyframes) = track.keyframes() {
            let mut keyframes_clone = keyframes.clone();

            // Add debug log
            println!(
                "Keyframe property list: {:?}",
                keyframes_clone.property_names()
            );
            if keyframes_clone.has_property("mute") {
                println!(
                    "mute property exists, current value: {:?}",
                    keyframes_clone.get_value_at("mute", TimePosition::from_seconds(0.0))
                );

                // Remove existing keyframes and then add
                keyframes_clone
                    .remove_keyframe("mute", TimePosition::from_seconds(0.0))
                    .unwrap();
                keyframes_clone
                    .add_keyframe(
                        "mute",
                        TimePosition::from_seconds(0.0),
                        1.0,
                        EasingFunction::Linear,
                    )
                    .unwrap();

                println!(
                    "New mute value: {:?}",
                    keyframes_clone.get_value_at("mute", TimePosition::from_seconds(0.0))
                );
            } else {
                println!("mute property does not exist");
                keyframes_clone.create_track_if_missing("mute").unwrap();
                keyframes_clone
                    .add_keyframe(
                        "mute",
                        TimePosition::from_seconds(0.0),
                        1.0,
                        EasingFunction::Linear,
                    )
                    .unwrap();
            }

            track.set_keyframes(Some(keyframes_clone));
        }

        // Add debug output (Before calling TrackCompositor::new)
        let mute_value = if let Some(track) = timeline_muted.get_track(track_id_muted) {
            if let Some(kf) = track.keyframes() {
                let value = kf.get_value_at("mute", TimePosition::from_seconds(0.0));
                println!("Test mute value: {:?}", value);
                value
            } else {
                None
            }
        } else {
            None
        };
        println!("Mute value: {:?}", mute_value);

        let compositor_muted = TrackCompositor::new(timeline_muted, Vec::new());
        let volume = compositor_muted.get_track_volume(track_id_muted);
        println!("Volume after mute: {}", volume);
        assert_eq!(volume, 0.0, "Should return 0.0 volume when muted");
    }

    #[test]
    fn test_get_track_volume_from_name() {
        let (timeline, track_id) = create_test_timeline_with_track("Background Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let volume = compositor.get_track_volume(track_id);
        assert_eq!(volume, 0.5, "Should return 0.5 volume for background track");

        let (timeline, track_id) = create_test_timeline_with_track("Effect Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let volume = compositor.get_track_volume(track_id);
        assert_eq!(volume, 0.7, "Should return 0.7 volume for effect track");

        let (timeline, track_id) = create_test_timeline_with_track("Main Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let volume = compositor.get_track_volume(track_id);
        assert_eq!(volume, 1.0, "Should return 1.0 volume for main track");
    }

    #[test]
    fn test_get_track_volume_default() {
        let (timeline, track_id) = create_test_timeline_with_track("Regular Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let volume = compositor.get_track_volume(track_id);
        assert_eq!(volume, 1.0, "Should return 1.0 volume as default");
    }
}
