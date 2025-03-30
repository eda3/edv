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

/// ブレンドモードの種類。
/// ビデオトラックの合成方法を指定します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// 通常の重ね合わせ（標準的なオーバーレイ）
    Normal,
    /// 加算合成（明るさが加算される）
    Add,
    /// 乗算合成（暗いピクセルが強調される）
    Multiply,
    /// スクリーン合成（明るいピクセルが強調される）
    Screen,
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
        if video_tracks.is_empty() {
            return String::new();
        }

        // フィルタパーツとオーバーレイチェーン用の変数
        let mut filter_parts = Vec::new();
        let mut overlay_chain = String::new();

        // マルチトラック関係を考慮したZ順序の決定
        let mut ordered_tracks: Vec<&PreparedTrack> = Vec::new();

        // トラック関係マネージャからソート順を取得
        let multi_track_manager = self.timeline.multi_track_manager();

        // まずは可視性のあるトラックだけをフィルタリング
        let visible_tracks: Vec<&PreparedTrack> = video_tracks
            .iter()
            .filter(|track| {
                // トラックが可視かどうかを確認（実際のデータから取得）
                // デフォルトは可視とする
                true
            })
            .copied()
            .collect();

        // 関係に基づいてトラックをソート
        // 下位レイヤーから上位レイヤーの順に処理（z-index が低いものから高いものへ）
        if !visible_tracks.is_empty() {
            // 未整理のトラックを追跡
            let mut remaining_tracks: Vec<&PreparedTrack> = visible_tracks.clone();

            // トラック関係に基づいて順序付け（シンプルなケース用）
            // 実装ではトラック関係マネージャを使って適切に順序付け

            // トラックIDからアルファベット順の単純なソート（実際の実装では置き換え）
            remaining_tracks.sort_by(|a, b| format!("{:?}", a.id).cmp(&format!("{:?}", b.id)));

            ordered_tracks = remaining_tracks;
        }

        // 各トラックを処理してフィルターグラフを構築
        for (i, track) in ordered_tracks.iter().enumerate() {
            let input_index = i; // 入力インデックスはFFmpegの入力順序に一致すると仮定

            // アルファチャンネルのサポートを確保（透明度のある合成のため）
            let format_filter = format!(
                "[{input_index}:v] format=yuva420p",
                input_index = input_index
            );

            // 出力サイズに合わせてスケーリング
            let scale_filter = format!(
                "{} ,scale={width}:{height},setsar=1",
                format_filter,
                width = config.width,
                height = config.height
            );

            // キーフレームとアニメーションのサポート追加
            let mut track_filters = scale_filter;

            // トラックのキーフレームをチェックして適用
            if let Some(track_obj) = self.timeline.get_track(track.id) {
                if let Some(keyframes) = track_obj.keyframes() {
                    // 不透明度キーフレームの処理
                    if keyframes.has_property("opacity") {
                        // タイムライン上の特定位置での不透明度を取得
                        // （現在の実装では簡略化のため中間位置で取得）
                        let opacity_pos =
                            TimePosition::from_seconds(track.duration.as_seconds() / 2.0);
                        let opacity_value = keyframes
                            .get_value_at("opacity", opacity_pos)
                            .unwrap_or(1.0); // デフォルトは完全不透明

                        track_filters =
                            format!("{},colorchannelmixer=aa={}", track_filters, opacity_value);
                    }

                    // 他のキーフレームアニメーションの処理
                    // 位置、回転、スケールなど
                    if keyframes.has_property("position_x") && keyframes.has_property("position_y")
                    {
                        // 位置キーフレームを適用
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
                        // スケールキーフレームを適用
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

            // 最終的なトラック出力ラベルを追加
            track_filters = format!("{} [v{}]", track_filters, i);
            filter_parts.push(track_filters);

            // 合成チェーンの構築
            if i == 0 {
                // 最初のトラックは基本レイヤーとして使用
                overlay_chain = format!("[v0]");
            } else {
                // トラック関係からブレンドモードを取得
                let blend_mode = self.get_blend_mode_for_track(track.id);

                // 選択されたブレンドモードに基づいて適切なオーバーレイフィルターを使用
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
                };

                let next_output = format!("[v{next}]", next = i + 1);
                filter_parts.push(format!("{} {}", overlay_filter, next_output));
                overlay_chain = next_output;
            }
        }

        // すべてのフィルター部分をセミコロンで連結
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
        // トラックからブレンドモードを取得
        if let Some(track) = self.timeline.get_track(track_id) {
            // まずはキーフレームからブレンドモードを確認
            if let Some(keyframes) = track.keyframes() {
                if keyframes.has_property("blend_mode") {
                    // ブレンドモードは数値で保存されていると仮定
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
                            _ => BlendMode::Normal, // デフォルト
                        };
                    }
                }
            }

            // トラックの種類に基づくデフォルトブレンドモード
            // 例: オーバーレイトラックにはスクリーンモードを使用
            let track_name = track.name().to_lowercase();
            if track_name.contains("overlay") || track_name.contains("オーバーレイ") {
                return BlendMode::Screen;
            } else if track_name.contains("effect") || track_name.contains("エフェクト") {
                return BlendMode::Add;
            } else if track_name.contains("shadow") || track_name.contains("影") {
                return BlendMode::Multiply;
            }
        }

        // デフォルトはNormal
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
        config: &RenderConfig,
    ) -> String {
        if audio_tracks.is_empty() {
            return String::new();
        }

        // フィルタパーツとミキシング入力用の変数
        let mut filter_parts = Vec::new();
        let mut amix_inputs = Vec::new();

        // マルチトラック関係と優先順位を考慮
        let multi_track_manager = self.timeline.multi_track_manager();

        // 各トラックを処理（順序は重要ではない - すべて混合される）
        for (i, track) in audio_tracks.iter().enumerate() {
            let input_index = i; // 入力インデックスはFFmpeg入力順序に一致すると仮定

            // ボリューム調整の適用
            let volume_value = self.get_track_volume(track.id);

            // オーディオのノーマライズとフォーマット設定
            let audio_filter = format!(
                "[{input_index}:a] aformat=sample_fmts=fltp:channel_layouts=stereo,volume={volume}",
                input_index = input_index,
                volume = volume_value
            );

            // タイムラインのフェードとエフェクトの処理
            let mut processed_filter = audio_filter;

            // キーフレームアニメーションの適用
            if let Some(track_obj) = self.timeline.get_track(track.id) {
                if let Some(keyframes) = track_obj.keyframes() {
                    // ボリュームキーフレームの処理
                    if keyframes.has_property("volume") {
                        // タイムライン上の特定位置でのボリューム値
                        // （現実的な実装では複数ポイントでのキーフレームをチェック）

                        // フェードイン/アウトの実装
                        let duration = track.duration.as_seconds();

                        // フェードイン（最初の1秒）
                        if duration > 1.0 {
                            processed_filter = format!("{},afade=t=in:st=0:d=1", processed_filter);
                        }

                        // フェードアウト（最後の1秒）
                        if duration > 2.0 {
                            processed_filter = format!(
                                "{},afade=t=out:st={}:d=1",
                                processed_filter,
                                duration - 1.0
                            );
                        }
                    }

                    // EQ設定のキーフレーム（低音/高音の調整など）
                    if keyframes.has_property("bass") || keyframes.has_property("treble") {
                        let bass = keyframes
                            .get_value_at("bass", TimePosition::from_seconds(0.0))
                            .unwrap_or(0.0);
                        let treble = keyframes
                            .get_value_at("treble", TimePosition::from_seconds(0.0))
                            .unwrap_or(0.0);

                        // 簡易的なEQ調整（実際の実装ではより詳細な設定）
                        if bass != 0.0 || treble != 0.0 {
                            processed_filter = format!(
                                "{},equalizer=f=100:t=h:width=200:g={}:f=10000:t=h:width=2000:g={}",
                                processed_filter, bass, treble
                            );
                        }
                    }
                }
            }

            // トラック間の関係性を考慮した処理
            // 例: ミュート設定やソロトラックの処理
            let mut is_muted = false;

            // ミュート状態の確認（実際の実装ではトラックプロパティから取得）
            if let Some(track_obj) = self.timeline.get_track(track.id) {
                if let Some(keyframes) = track_obj.keyframes() {
                    if let Some(mute_value) =
                        keyframes.get_value_at("mute", TimePosition::from_seconds(0.0))
                    {
                        is_muted = mute_value > 0.5; // 0.5以上ならミュート
                    }
                }
            }

            // ミュートトラックの場合はボリュームを0に設定
            if is_muted {
                processed_filter = format!("{},volume=0", processed_filter);
            }

            // 出力ラベルを追加
            processed_filter = format!("{} [a{}]", processed_filter, i);
            filter_parts.push(processed_filter);
            amix_inputs.push(format!("[a{}]", i));
        }

        // 複数のトラックがある場合はamixフィルターを追加
        if audio_tracks.len() > 1 {
            // 高度なミキシングパラメータ
            // duration=longest: 最も長いトラックに合わせる
            // normalize=0: ボリュームを正規化しない（手動設定を優先）
            // dropout_transition: トラック終了時のフェードアウト時間
            let amix_filter = format!(
                "{} amix=inputs={}:duration=longest:normalize=0:dropout_transition=0.5 [aout]",
                amix_inputs.join(""),
                audio_tracks.len()
            );
            filter_parts.push(amix_filter);
        } else if !audio_tracks.is_empty() {
            // 単一トラックの場合は直接マッピング
            filter_parts.push(format!("{} asetpts=PTS-STARTPTS [aout]", amix_inputs[0]));
        }

        // すべてのフィルター部分をセミコロンで連結
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
        // トラックからボリューム設定を取得
        if let Some(track) = self.timeline.get_track(track_id) {
            // キーフレームからボリューム値を確認
            if let Some(keyframes) = track.keyframes() {
                // ミュート状態を最初に確認
                if keyframes.has_property("mute") {
                    if let Some(mute_value) =
                        keyframes.get_value_at("mute", TimePosition::from_seconds(0.0))
                    {
                        if mute_value > 0.5 {
                            return 0.0; // ミュート時はボリューム0
                        }
                    }
                }

                // 次に直接のボリュームプロパティを確認
                if keyframes.has_property("volume") {
                    if let Some(volume) =
                        keyframes.get_value_at("volume", TimePosition::from_seconds(0.0))
                    {
                        // 値が有効な範囲内かチェック（0.0〜2.0）
                        if volume >= 0.0 {
                            return volume.min(2.0); // 最大2.0（200%）に制限
                        }
                    }
                }
            }

            // トラックの種類に基づくデフォルトボリューム
            let track_name = track.name().to_lowercase();
            if track_name.contains("background") || track_name.contains("バックグラウンド")
            {
                return 0.5; // 背景は少し小さめのボリューム
            } else if track_name.contains("effect") || track_name.contains("エフェクト") {
                return 0.7; // エフェクトは控えめのボリューム
            } else if track_name.contains("main") || track_name.contains("メイン") {
                return 1.0; // メイントラックは通常ボリューム
            }
        }

        // デフォルトボリュームは1.0（100%）
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
            // ビデオフィルターがある場合はセミコロンで区切る
            if !complex_filter.is_empty() {
                complex_filter.push(';');
            }
            complex_filter.push_str(&self.generate_audio_filtergraph(&audio_tracks, config));
        }

        // フィルターが存在する場合、コマンドにフィルターを追加
        if !complex_filter.is_empty() {
            ffmpeg.add_complex_filter(&complex_filter);

            // フィルター出力のマッピング
            if !video_tracks.is_empty() {
                ffmpeg.add_arg("-map", &format!("[v{}]", video_tracks.len()));
            }
            if !audio_tracks.is_empty() {
                ffmpeg.add_arg("-map", "[aout]");
            }
        } else {
            // フィルターがない場合、デフォルトのマッピングを使用
            if !video_tracks.is_empty() {
                ffmpeg.add_arg("-map", "0:v");
            }
            if !audio_tracks.is_empty() {
                ffmpeg.add_arg("-map", "0:a");
            }
        }

        // 字幕トラックのマッピング
        if !subtitle_tracks.is_empty() {
            // 字幕トラックのマッピングロジック（簡略化）
            ffmpeg.add_arg("-map", "0:s");
        }

        // 出力フォーマットと品質設定
        ffmpeg.set_output(output_path);

        // ビデオコーデック設定
        if !video_tracks.is_empty() {
            ffmpeg.add_arg("-c:v", config.video_codec.to_ffmpeg_codec());
            ffmpeg.add_arg("-pix_fmt", "yuv420p"); // 互換性のための標準ピクセルフォーマット

            // 品質設定
            if config.video_codec != VideoCodec::Copy {
                // 例: H.264 / H.265 用の CRF 設定
                if config.video_codec == VideoCodec::H264 || config.video_codec == VideoCodec::H265
                {
                    ffmpeg.add_arg("-crf", "23"); // デフォルトの品質値
                }
            }

            // プリセット設定（エンコード速度と圧縮率のバランス）
            if config.video_codec == VideoCodec::H264 || config.video_codec == VideoCodec::H265 {
                ffmpeg.add_arg("-preset", "medium");
            }

            // フレームレート設定
            ffmpeg.add_arg("-r", &config.frame_rate.to_string());
        }

        // オーディオコーデック設定
        if !audio_tracks.is_empty() {
            ffmpeg.add_arg("-c:a", config.audio_codec.to_ffmpeg_codec());
            // オーディオ品質設定
            if config.audio_codec != AudioCodec::Copy {
                ffmpeg.add_arg("-b:a", "192k");
            }
        }

        // 字幕設定
        if !subtitle_tracks.is_empty() {
            ffmpeg.add_arg("-c:s", "mov_text"); // 互換性のある字幕フォーマット
        }

        // 複雑なタイムラインのための最適化
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utility::time::{Duration, TimePosition};

    // テスト用のタイムラインとトラックを作成するヘルパー関数
    fn create_test_timeline_with_track(
        track_name: &str,
        add_keyframes: bool,
    ) -> (Timeline, TrackId) {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        let track = timeline.get_track_mut(track_id).unwrap();
        track.set_name(track_name);

        if add_keyframes {
            // Duration付きでKeyframeAnimationを作成
            let mut keyframes = KeyframeAnimation::new(Duration::from_seconds(10.0));

            // 各プロパティのトラックを作成
            keyframes.create_track_if_missing("blend_mode").unwrap();
            keyframes.create_track_if_missing("volume").unwrap();
            keyframes.create_track_if_missing("mute").unwrap();

            // キーフレームを追加（イージング関数付き）
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

            // Option<KeyframeAnimation>として渡す
            track.set_keyframes(Some(keyframes));
        }

        (timeline, track_id)
    }

    #[test]
    fn test_get_blend_mode_for_track_from_keyframes() {
        let (timeline, track_id) = create_test_timeline_with_track("Test Track", true);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(
            blend_mode,
            BlendMode::Screen,
            "Should return Screen blend mode from keyframes"
        );
    }

    #[test]
    fn test_get_blend_mode_for_track_from_name() {
        let (timeline, track_id) = create_test_timeline_with_track("Overlay Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(
            blend_mode,
            BlendMode::Screen,
            "Should return Screen blend mode based on track name"
        );

        let (timeline, track_id) = create_test_timeline_with_track("Effect Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(
            blend_mode,
            BlendMode::Add,
            "Should return Add blend mode based on track name"
        );

        let (timeline, track_id) = create_test_timeline_with_track("Shadow Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(
            blend_mode,
            BlendMode::Multiply,
            "Should return Multiply blend mode based on track name"
        );
    }

    #[test]
    fn test_get_blend_mode_for_track_default() {
        let (timeline, track_id) = create_test_timeline_with_track("Regular Track", false);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let blend_mode = compositor.get_blend_mode_for_track(track_id);
        assert_eq!(
            blend_mode,
            BlendMode::Normal,
            "Should return Normal blend mode as default"
        );
    }

    #[test]
    fn test_get_track_volume_from_keyframes() {
        let (timeline, track_id) = create_test_timeline_with_track("Test Track", true);
        let compositor = TrackCompositor::new(timeline, Vec::new());

        let volume = compositor.get_track_volume(track_id);
        assert_eq!(volume, 1.5, "Should return 1.5 volume from keyframes");

        // ミュート状態のテスト - 新しいタイムラインを作成
        let (mut timeline_muted, track_id_muted) =
            create_test_timeline_with_track("Test Track", true);
        let track = timeline_muted.get_track_mut(track_id_muted).unwrap();
        if let Some(keyframes) = track.keyframes() {
            let mut keyframes_clone = keyframes.clone();

            // デバッグログを追加
            println!(
                "キーフレーム プロパティ一覧: {:?}",
                keyframes_clone.property_names()
            );
            if keyframes_clone.has_property("mute") {
                println!(
                    "mute property exists, current value: {:?}",
                    keyframes_clone.get_value_at("mute", TimePosition::from_seconds(0.0))
                );

                // 既存のキーフレームを削除してから追加する（確実な方法）
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
                    "新しいmute値: {:?}",
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

        // デバッグ出力を追加（TrackCompositor::newを呼び出す前）
        let mute_value = if let Some(track) = timeline_muted.get_track(track_id_muted) {
            if let Some(kf) = track.keyframes() {
                let value = kf.get_value_at("mute", TimePosition::from_seconds(0.0));
                println!("テスト時のmute値: {:?}", value);
                value
            } else {
                None
            }
        } else {
            None
        };
        println!("ミュート値: {:?}", mute_value);

        let compositor_muted = TrackCompositor::new(timeline_muted, Vec::new());
        let volume = compositor_muted.get_track_volume(track_id_muted);
        println!("ミュート後のボリューム: {}", volume);
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
