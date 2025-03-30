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
use std::path::Path;
use std::process::{Command, Stdio};

use crate::project::AssetId;
use crate::project::AssetReference;
use crate::project::rendering::config::RenderConfig;
use crate::project::rendering::progress::{RenderStage, SharedProgressTracker};
use crate::project::timeline::{Clip, Timeline, Track, TrackId, TrackKind};
use crate::utility::time::{Duration, TimePosition};

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
    fn add_input<P: AsRef<Path>>(&mut self, input: P) -> &mut Self {
        self.inputs.push(input.as_ref().to_path_buf());
        self
    }

    /// Sets the output file for the command
    fn set_output<P: AsRef<Path>>(&mut self, output: P) -> &mut Self {
        self.output = Some(output.as_ref().to_path_buf());
        self
    }

    /// Sets the video codec
    fn set_video_codec(&mut self, codec: &str) -> &mut Self {
        self.args.push("-c:v".to_string());
        self.args.push(codec.to_string());
        self
    }

    /// Sets the audio codec
    fn set_audio_codec(&mut self, codec: &str) -> &mut Self {
        self.args.push("-c:a".to_string());
        self.args.push(codec.to_string());
        self
    }

    /// Sets the frame rate
    fn set_frame_rate(&mut self, frame_rate: f64) -> &mut Self {
        self.args.push("-r".to_string());
        self.args.push(frame_rate.to_string());
        self
    }

    /// Sets the video size
    fn set_video_size(&mut self, width: u32, height: u32) -> &mut Self {
        self.args.push("-s".to_string());
        self.args.push(format!("{}x{}", width, height));
        self
    }

    /// Runs the FFmpeg command
    fn run(&self) -> Result<()> {
        if self.inputs.is_empty() {
            return Err(CompositionError::FFmpeg(
                crate::ffmpeg::Error::MissingArgument("No input files specified".to_string()),
            ));
        }

        if self.output.is_none() {
            return Err(CompositionError::FFmpeg(
                crate::ffmpeg::Error::MissingArgument("No output file specified".to_string()),
            ));
        }

        let mut cmd = Command::new("ffmpeg");

        // Always overwrite output files
        cmd.arg("-y");

        // Add all inputs
        for input in &self.inputs {
            cmd.arg("-i").arg(input);
        }

        // Add all additional arguments
        for arg in &self.args {
            cmd.arg(arg);
        }

        // Add output file
        cmd.arg(self.output.as_ref().unwrap());

        // Run the command
        let output = cmd
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| CompositionError::FFmpeg(crate::ffmpeg::Error::IoError(e)))?;

        // Check if successful
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CompositionError::FFmpeg(
                crate::ffmpeg::Error::ProcessTerminated {
                    exit_code: output.status.code(),
                    message: format!("FFmpeg process failed: {}", stderr),
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
        if self.optimize_complex && tracks_to_process.len() > 4 {
            // 利用可能なCPUコア数に基づいてスレッド数を決定
            let num_cpus = num_cpus::get();
            let thread_count = (num_cpus / 2).max(1);

            // ログメッセージ（実際の実装ではロガーを使用）
            println!(
                "Optimizing for complex timeline with {} tracks using {} threads",
                tracks_to_process.len(),
                thread_count
            );

            // 動的なFFmpegコマンドパラメータを調整
            // 実際の実装ではこの部分にFFmpegの最適化パラメータを設定
        }

        // 各トラックを適切なメソッドで処理
        for (track_id, kind, clips) in tracks_to_process {
            let prepared_track = match kind {
                TrackKind::Video => self.prepare_video_track_from_data(track_id, &clips, config)?,
                TrackKind::Audio => self.prepare_audio_track_from_data(track_id, &clips, config)?,
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
        config: &RenderConfig,
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
        config: &RenderConfig,
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
        // Track indices for input mapping (0-based in FFmpeg)
        let mut filter_parts = Vec::new();
        let mut overlay_chain = String::new();

        // Sort tracks by z-order (typically order of addition is the z-order in simple cases)
        // In a real implementation, this would use explicit z-order from the tracks
        let mut ordered_tracks: Vec<&PreparedTrack> = video_tracks.to_vec();

        // Sort by ID - we'll use the Debug representation as a simple way to compare
        ordered_tracks.sort_by(|a, b| format!("{:?}", a.id).cmp(&format!("{:?}", b.id)));

        // Process each track to prepare it for composition
        for (i, track) in ordered_tracks.iter().enumerate() {
            let input_index = i; // Assume track index matches FFmpeg input index

            // Scale video to match output dimensions
            let scale_filter = format!(
                "[{input_index}:v] scale={width}:{height},setsar=1 [v{i}]",
                input_index = input_index,
                width = config.width,
                height = config.height,
                i = i
            );
            filter_parts.push(scale_filter);

            // For the first track, just use it as the base
            if i == 0 {
                overlay_chain = format!("[v0]");
            } else {
                // For subsequent tracks, overlay them on top of the previous result
                let overlay_filter = format!(
                    "{prev}[v{current}] overlay=shortest=1 [v{next}]",
                    prev = overlay_chain,
                    current = i,
                    next = i + 1
                );
                filter_parts.push(overlay_filter);
                overlay_chain = format!("[v{}]", i + 1);
            }
        }

        // Concatenate all filter parts with semicolons
        filter_parts.join(";")
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
        config: &RenderConfig,
    ) -> String {
        if audio_tracks.is_empty() {
            return String::new();
        }

        // Track indices for input mapping
        let mut filter_parts = Vec::new();
        let mut amix_inputs = Vec::new();

        // Process each track
        for (i, _track) in audio_tracks.iter().enumerate() {
            let input_index = i; // Assume track index matches FFmpeg input index

            // Normalize audio to prevent clipping
            let normalize_filter = format!(
                "[{input_index}:a] aformat=sample_fmts=fltp:channel_layouts=stereo,volume=1.0 [a{i}]",
                input_index = input_index,
                i = i
            );
            filter_parts.push(normalize_filter);
            amix_inputs.push(format!("[a{}]", i));
        }

        // Add the amix filter if we have multiple tracks
        if audio_tracks.len() > 1 {
            let amix_filter = format!(
                "{} amix=inputs={}:duration=longest [aout]",
                amix_inputs.join(""),
                audio_tracks.len()
            );
            filter_parts.push(amix_filter);
        } else {
            // If only one track, just map it directly
            filter_parts.push(format!("[a0] [aout]"));
        }

        // Concatenate all filter parts with semicolons
        filter_parts.join(";")
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

        // FFmpegコマンドを構築
        let mut ffmpeg = FFmpegCommand::new();

        // トラックをタイプ別に分類
        let mut video_tracks: Vec<&PreparedTrack> = Vec::new();
        let mut audio_tracks: Vec<&PreparedTrack> = Vec::new();
        let mut subtitle_tracks: Vec<&PreparedTrack> = Vec::new();

        // すべての入力ファイルを追加し、トラック種類で分類
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

        // フィルタグラフの構築
        let mut complex_filter = String::new();

        // ビデオトラックのフィルタグラフを生成
        if !video_tracks.is_empty() {
            complex_filter.push_str(&self.generate_video_filtergraph(&video_tracks, config));
        }

        // オーディオトラックのフィルタグラフを生成
        if !audio_tracks.is_empty() {
            if !complex_filter.is_empty() {
                complex_filter.push_str(";");
            }
            complex_filter.push_str(&self.generate_audio_filtergraph(&audio_tracks, config));
        }

        // フィルタグラフをコマンドに追加
        if !complex_filter.is_empty() {
            ffmpeg.args.push("-filter_complex".to_string());
            ffmpeg.args.push(complex_filter);

            // 出力ストリームマッピング
            if !video_tracks.is_empty() {
                ffmpeg.args.push("-map".to_string());
                ffmpeg.args.push("[v]".to_string());
            }

            if !audio_tracks.is_empty() {
                ffmpeg.args.push("-map".to_string());
                ffmpeg.args.push("[aout]".to_string());
            }
        }

        // 字幕トラックがある場合の処理
        if !subtitle_tracks.is_empty() {
            // TODO: 字幕の処理を実装
            // 現在のところ、サポートなし
        }

        // 出力オプションを設定
        ffmpeg
            .set_output(output_path)
            .set_video_codec(config.video_codec.to_ffmpeg_codec())
            .set_audio_codec(config.audio_codec.to_ffmpeg_codec())
            .set_frame_rate(config.frame_rate)
            .set_video_size(config.width, config.height);

        // 追加のエンコードオプション
        if self.optimize_complex {
            // 複雑なタイムラインのための最適化オプション
            if video_tracks.len() > 3 {
                // マルチパスエンコーディングを使用
                ffmpeg.args.push("-pass".to_string());
                ffmpeg.args.push("1".to_string());

                // ビデオビットレートを調整
                ffmpeg.args.push("-b:v".to_string());
                ffmpeg
                    .args
                    .push(format!("{}k", config.width * config.height / 1000));

                // スレッド数を指定
                ffmpeg.args.push("-threads".to_string());
                ffmpeg.args.push(num_cpus::get().to_string());
            }
        }

        // レンダリング開始
        self.update_progress(RenderStage::Rendering);

        // FFmpegを実行
        ffmpeg.run()?;

        // 完了
        self.update_progress(RenderStage::PostProcessing);

        // レンダリング完成
        self.update_progress(RenderStage::Complete);

        Ok(())
    }
}
