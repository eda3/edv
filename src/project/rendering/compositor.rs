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

use crate::project::AssetId;
use crate::project::AssetReference;
use crate::project::rendering::config::RenderConfig;
use crate::project::rendering::progress::{RenderStage, SharedProgressTracker};
use crate::project::timeline::{Clip, Timeline, Track, TrackId, TrackKind};
use crate::utility::time::{Duration, TimePosition};

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
        }
    }

    /// Sets the progress tracker for the composition.
    pub fn set_progress_tracker(&mut self, progress: SharedProgressTracker) {
        self.progress = Some(progress);
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
        let timeline_duration = self.calculate_timeline_duration();

        // 先にすべてのトラックとその種類を収集
        let tracks_to_process: Vec<(TrackId, TrackKind, Vec<Clip>)> = self
            .timeline
            .get_tracks()
            .iter()
            .filter(|track| !track.get_clips().is_empty())
            .map(|track| (track.id(), track.kind(), track.get_clips().to_vec()))
            .collect();

        // 各トラックを処理
        for (track_id, track_kind, clips) in tracks_to_process {
            if self.is_cancelled() {
                return Ok(prepared_tracks); // Early return if cancelled
            }

            // Prepare the track based on its kind
            let prepared = match track_kind {
                TrackKind::Video => self.prepare_video_track_from_data(track_id, &clips, config)?,
                TrackKind::Audio => self.prepare_audio_track_from_data(track_id, &clips, config)?,
                TrackKind::Subtitle => {
                    self.prepare_subtitle_track_from_data(track_id, &clips, config)?
                }
            };

            prepared_tracks.insert(track_id, prepared);
        }

        Ok(prepared_tracks)
    }

    /// Calculates the total duration of the timeline.
    fn calculate_timeline_duration(&self) -> Duration {
        let mut max_duration = Duration::from_seconds(0.0);

        for track in self.timeline.get_tracks() {
            let track_end = track
                .get_clips()
                .iter()
                .fold(TimePosition::from_seconds(0.0), |acc, clip| {
                    acc.max(clip.position() + clip.duration())
                });

            max_duration =
                Duration::from_seconds(f64::max(max_duration.as_seconds(), track_end.as_seconds()));
        }

        max_duration
    }

    /// Prepares a video track for composition using clip data.
    fn prepare_video_track_from_data(
        &mut self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // Create intermediate file for the rendered track
        let intermediate = IntermediateFile::new("mp4")?;

        // Render the video track to the intermediate file
        // (This would involve using FFmpeg to render the clips to a single file)
        // ...

        // For now, we're just creating a placeholder
        let prepared = PreparedTrack {
            id: track_id,
            kind: TrackKind::Video,
            file: Some(intermediate),
            clips: clips.to_vec(),
            duration: self.calculate_track_duration_from_clips(clips),
        };

        Ok(prepared)
    }

    /// Prepares an audio track for composition using clip data.
    fn prepare_audio_track_from_data(
        &mut self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // Create intermediate file for the rendered track
        let intermediate = IntermediateFile::new("wav")?;

        // Render the audio track to the intermediate file
        // (This would involve using FFmpeg to render the clips to a single file)
        // ...

        // For now, we're just creating a placeholder
        let prepared = PreparedTrack {
            id: track_id,
            kind: TrackKind::Audio,
            file: Some(intermediate),
            clips: clips.to_vec(),
            duration: self.calculate_track_duration_from_clips(clips),
        };

        Ok(prepared)
    }

    /// Prepares a subtitle track for composition using clip data.
    fn prepare_subtitle_track_from_data(
        &mut self,
        track_id: TrackId,
        clips: &[Clip],
        _config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        // Create intermediate file for the rendered track
        let intermediate = IntermediateFile::new("srt")?;

        // Render the subtitle track to the intermediate file
        // (This would involve converting the subtitle clips to a single subtitle file)
        // ...

        // For now, we're just creating a placeholder
        let prepared = PreparedTrack {
            id: track_id,
            kind: TrackKind::Subtitle,
            file: Some(intermediate),
            clips: clips.to_vec(),
            duration: self.calculate_track_duration_from_clips(clips),
        };

        Ok(prepared)
    }

    /// Calculates the duration from a collection of clips.
    fn calculate_track_duration_from_clips(&self, clips: &[Clip]) -> Duration {
        clips.iter().fold(Duration::from_seconds(0.0), |acc, clip| {
            let clip_end = clip.position() + clip.duration();
            Duration::from_seconds(f64::max(acc.as_seconds(), clip_end.as_seconds()))
        })
    }

    /// Composites prepared tracks into the final output.
    ///
    /// # Arguments
    ///
    /// * `prepared_tracks` - The prepared tracks to composite
    /// * `config` - The rendering configuration
    /// * `output_path` - The path for the final output file
    ///
    /// # Returns
    ///
    /// `Ok(())` if the composition was successful, or an error if it failed.
    fn composite_tracks(
        &mut self,
        prepared_tracks: HashMap<TrackId, PreparedTrack>,
        config: &RenderConfig,
        output_path: &Path,
    ) -> Result<()> {
        self.update_progress(RenderStage::Muxing);

        if prepared_tracks.is_empty() {
            return Err(CompositionError::IncompatibleTracks(
                "No tracks to composite".to_string(),
            ));
        }

        // Group tracks by kind
        let mut video_tracks = Vec::new();
        let mut audio_tracks = Vec::new();
        let mut subtitle_tracks = Vec::new();

        for (_, track) in prepared_tracks {
            match track.kind {
                TrackKind::Video => video_tracks.push(track),
                TrackKind::Audio => audio_tracks.push(track),
                TrackKind::Subtitle => subtitle_tracks.push(track),
            }
        }

        // Build FFmpeg command for composition
        // (This would combine all tracks into the final output file)
        // ...

        // For now, we'll just simulate the composition
        self.update_progress(RenderStage::Finalizing);

        // In a real implementation, we'd run the FFmpeg command here
        // and handle its output/errors

        Ok(())
    }

    /// Composes the timeline into a rendered output file.
    ///
    /// # Arguments
    ///
    /// * `config` - The rendering configuration
    ///
    /// # Returns
    ///
    /// `Ok(())` if the composition was successful, or an error if it failed.
    pub fn compose(&mut self, config: &RenderConfig) -> Result<()> {
        // Validate the configuration
        if let Err(err) = config.validate() {
            return Err(CompositionError::IncompatibleTracks(err));
        }

        // Prepare the tracks
        let prepared_tracks = self.prepare_tracks(config)?;

        if self.is_cancelled() {
            return Err(CompositionError::FFmpeg(
                crate::ffmpeg::Error::ExecutionError("Rendering cancelled by user".to_string()),
            ));
        }

        // Composite the prepared tracks
        self.composite_tracks(prepared_tracks, config, &config.output_path)?;

        // Mark the composition as complete
        if let Some(progress) = &self.progress {
            progress.complete();
        }

        Ok(())
    }

    /// Prepares a video track for composition.
    fn prepare_video_track(
        &mut self,
        track: &Track,
        config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        self.prepare_video_track_from_data(track.id(), track.get_clips(), config)
    }

    /// Prepares an audio track for composition.
    fn prepare_audio_track(
        &mut self,
        track: &Track,
        config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        self.prepare_audio_track_from_data(track.id(), track.get_clips(), config)
    }

    /// Prepares a subtitle track for composition.
    fn prepare_subtitle_track(
        &mut self,
        track: &Track,
        config: &RenderConfig,
    ) -> Result<PreparedTrack> {
        self.prepare_subtitle_track_from_data(track.id(), track.get_clips(), config)
    }

    /// Calculates the duration of a track.
    fn calculate_track_duration(&self, track: &Track) -> Duration {
        self.calculate_track_duration_from_clips(track.get_clips())
    }
}
