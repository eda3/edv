/// Timeline management functionality.
///
/// This module provides functionality for creating, editing, and managing
/// video timelines, including tracks, clips, and multi-track relationships.
use uuid::Uuid;

pub mod multi_track;

use crate::utility::time::{Duration, TimePosition};

/// Error types specific to timeline operations.
#[derive(Debug, thiserror::Error)]
pub enum TimelineError {
    /// Error when a track is not found.
    #[error("Track not found: {0}")]
    TrackNotFound(TrackId),

    /// Error when a clip is not found.
    #[error("Clip not found in track {track}: {clip}")]
    ClipNotFound {
        track: TrackId,
        clip: crate::project::ClipId,
    },

    /// Error when clips overlap.
    #[error("Clip overlap at position {position}")]
    ClipOverlap { position: TimePosition },

    /// Error during multi-track operations.
    #[error("Multi-track error: {0}")]
    MultiTrack(#[from] multi_track::MultiTrackError),

    /// Error when the operation would result in an invalid state.
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Type alias for timeline operation results.
pub type Result<T> = std::result::Result<T, TimelineError>;

/// Unique identifier for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackId(Uuid);

impl TrackId {
    /// Creates a new random track ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for TrackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of tracks in a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    /// Video track.
    Video,
    /// Audio track.
    Audio,
    /// Subtitle track.
    Subtitle,
}

impl TrackKind {
    /// Gets a string representation of the track kind.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Subtitle => "Subtitle",
        }
    }
}

impl std::fmt::Display for TrackKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A clip in a timeline track.
#[derive(Debug, Clone)]
pub struct Clip {
    /// Unique identifier for the clip.
    id: crate::project::ClipId,

    /// ID of the asset used in the clip.
    asset_id: crate::project::AssetId,

    /// Position of the clip in the timeline.
    position: TimePosition,

    /// Duration of the clip.
    duration: Duration,

    /// Start position in the source asset.
    source_start: TimePosition,

    /// End position in the source asset.
    source_end: TimePosition,
}

impl Clip {
    /// Creates a new clip.
    ///
    /// # Arguments
    ///
    /// * `id` - The clip ID
    /// * `asset_id` - The asset ID
    /// * `position` - The position in the timeline
    /// * `duration` - The duration of the clip
    /// * `source_start` - The start position in the source asset
    /// * `source_end` - The end position in the source asset
    #[must_use]
    pub fn new(
        id: crate::project::ClipId,
        asset_id: crate::project::AssetId,
        position: TimePosition,
        duration: Duration,
        source_start: TimePosition,
        source_end: TimePosition,
    ) -> Self {
        Self {
            id,
            asset_id,
            position,
            duration,
            source_start,
            source_end,
        }
    }

    /// Gets the ID of the clip.
    #[must_use]
    pub fn id(&self) -> crate::project::ClipId {
        self.id
    }

    /// Gets the ID of the asset used in the clip.
    #[must_use]
    pub fn asset_id(&self) -> crate::project::AssetId {
        self.asset_id
    }

    /// Gets the position of the clip in the timeline.
    #[must_use]
    pub fn position(&self) -> TimePosition {
        self.position
    }

    /// Gets the end position of the clip in the timeline.
    #[must_use]
    pub fn end_position(&self) -> TimePosition {
        self.position + self.duration
    }

    /// Gets the duration of the clip.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Gets the start position in the source asset.
    #[must_use]
    pub fn source_start(&self) -> TimePosition {
        self.source_start
    }

    /// Gets the end position in the source asset.
    #[must_use]
    pub fn source_end(&self) -> TimePosition {
        self.source_end
    }

    /// Sets the position of the clip in the timeline.
    pub fn set_position(&mut self, position: TimePosition) {
        self.position = position;
    }

    /// Sets the duration of the clip.
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Sets the start position in the source asset.
    pub fn set_source_start(&mut self, start: TimePosition) {
        self.source_start = start;
    }

    /// Sets the end position in the source asset.
    pub fn set_source_end(&mut self, end: TimePosition) {
        self.source_end = end;
    }

    /// Checks if this clip overlaps with another clip.
    ///
    /// # Arguments
    ///
    /// * `other` - The other clip to check for overlap
    #[must_use]
    pub fn overlaps_with(&self, other: &Self) -> bool {
        self.position < other.end_position() && other.position < self.end_position()
    }
}

/// A track in a timeline.
#[derive(Debug, Clone)]
pub struct Track {
    /// Unique identifier for the track.
    id: TrackId,

    /// Type of the track.
    kind: TrackKind,

    /// Name of the track.
    name: String,

    /// Clips in the track, ordered by position.
    clips: Vec<Clip>,

    /// Whether the track is muted.
    muted: bool,

    /// Whether the track is locked for editing.
    locked: bool,
}

impl Track {
    /// Creates a new empty track.
    ///
    /// # Arguments
    ///
    /// * `id` - The track ID
    /// * `kind` - The track kind
    #[must_use]
    pub fn new(id: TrackId, kind: TrackKind) -> Self {
        Self {
            id,
            kind,
            name: format!("Track {}", kind),
            clips: Vec::new(),
            muted: false,
            locked: false,
        }
    }

    /// Gets the ID of the track.
    #[must_use]
    pub fn id(&self) -> TrackId {
        self.id
    }

    /// Gets the kind of the track.
    #[must_use]
    pub fn kind(&self) -> TrackKind {
        self.kind
    }

    /// Gets the name of the track.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name of the track.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Gets whether the track is muted.
    #[must_use]
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Sets whether the track is muted.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Gets whether the track is locked for editing.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Sets whether the track is locked for editing.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Gets the clips in the track.
    #[must_use]
    pub fn get_clips(&self) -> &[Clip] {
        &self.clips
    }

    /// Gets a mutable reference to a clip by its ID.
    pub fn get_clip_mut(&mut self, clip_id: crate::project::ClipId) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|clip| clip.id() == clip_id)
    }

    /// Gets a reference to a clip by its ID.
    #[must_use]
    pub fn get_clip(&self, clip_id: crate::project::ClipId) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.id() == clip_id)
    }

    /// Adds a clip to the track.
    ///
    /// # Arguments
    ///
    /// * `clip` - The clip to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the clip was added successfully, or an error if the clip
    /// overlaps with an existing clip.
    ///
    /// # Errors
    ///
    /// Returns an error if the clip overlaps with an existing clip.
    pub fn add_clip(&mut self, clip: Clip) -> Result<()> {
        // Check for overlap with existing clips
        for existing in &self.clips {
            if existing.overlaps_with(&clip) {
                return Err(TimelineError::ClipOverlap {
                    position: clip.position(),
                });
            }
        }

        self.clips.push(clip);
        // Sort clips by position
        self.clips
            .sort_by(|a, b| a.position().partial_cmp(&b.position()).unwrap());
        Ok(())
    }

    /// Removes a clip from the track.
    ///
    /// # Arguments
    ///
    /// * `clip_id` - The ID of the clip to remove
    ///
    /// # Returns
    ///
    /// `true` if the clip was found and removed, `false` otherwise.
    pub fn remove_clip(&mut self, clip_id: crate::project::ClipId) -> bool {
        let len = self.clips.len();
        self.clips.retain(|clip| clip.id() != clip_id);
        self.clips.len() < len
    }

    /// Gets the duration of the track.
    #[must_use]
    pub fn duration(&self) -> Duration {
        if self.clips.is_empty() {
            return Duration::zero();
        }

        // Find the clip that ends last
        let end = self
            .clips
            .iter()
            .map(|clip| clip.end_position())
            .max()
            .unwrap_or_else(TimePosition::zero);

        Duration::from_seconds(end.as_seconds())
    }
}

/// A timeline containing multiple tracks.
#[derive(Debug, Clone)]
pub struct Timeline {
    /// Tracks in the timeline.
    tracks: Vec<Track>,

    /// Multi-track manager for handling track relationships.
    multi_track_manager: multi_track::MultiTrackManager,
}

impl Timeline {
    /// Creates a new empty timeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            multi_track_manager: multi_track::MultiTrackManager::new(),
        }
    }

    /// Gets a reference to all tracks in the timeline.
    #[must_use]
    pub fn get_tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Gets a mutable reference to a track by its ID.
    pub fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|track| track.id() == track_id)
    }

    /// Gets a reference to a track by its ID.
    #[must_use]
    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id() == track_id)
    }

    /// Checks if a track with the given ID exists.
    #[must_use]
    pub fn has_track(&self, track_id: TrackId) -> bool {
        self.tracks.iter().any(|track| track.id() == track_id)
    }

    /// Adds a new track to the timeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of track to add
    ///
    /// # Returns
    ///
    /// The ID of the added track.
    pub fn add_track(&mut self, kind: TrackKind) -> TrackId {
        let id = TrackId::new();
        let track = Track::new(id, kind);
        self.tracks.push(track);
        id
    }

    /// Removes a track from the timeline.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to remove
    ///
    /// # Returns
    ///
    /// `Ok(())` if the track was removed successfully, or an error if the track
    /// does not exist or has dependencies.
    ///
    /// # Errors
    ///
    /// Returns an error if the track does not exist or if there are other tracks
    /// that depend on it.
    pub fn remove_track(&mut self, track_id: TrackId) -> Result<()> {
        if !self.has_track(track_id) {
            return Err(TimelineError::TrackNotFound(track_id));
        }

        // Remove track relationships
        self.multi_track_manager.remove_track(track_id)?;

        // Remove the track
        self.tracks.retain(|track| track.id() != track_id);

        Ok(())
    }

    /// Adds a clip to a track.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to add the clip to
    /// * `clip` - The clip to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the clip was added successfully, or an error if the track
    /// does not exist or the clip overlaps with an existing clip.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The track does not exist
    /// * The clip overlaps with an existing clip in the track
    pub fn add_clip(&mut self, track_id: TrackId, clip: Clip) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;

        track.add_clip(clip)?;

        Ok(())
    }

    /// Removes a clip from a track.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to remove the clip from
    /// * `clip_id` - The ID of the clip to remove
    ///
    /// # Returns
    ///
    /// `Ok(())` if the clip was removed successfully, or an error if the track
    /// or clip does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The track does not exist
    /// * The clip does not exist in the track
    pub fn remove_clip(
        &mut self,
        track_id: TrackId,
        clip_id: crate::project::ClipId,
    ) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;

        if !track.remove_clip(clip_id) {
            return Err(TimelineError::ClipNotFound {
                track: track_id,
                clip: clip_id,
            });
        }

        Ok(())
    }

    /// Gets the multi-track manager.
    #[must_use]
    pub fn multi_track_manager(&self) -> &multi_track::MultiTrackManager {
        &self.multi_track_manager
    }

    /// Gets a mutable reference to the multi-track manager.
    pub fn multi_track_manager_mut(&mut self) -> &mut multi_track::MultiTrackManager {
        &mut self.multi_track_manager
    }

    /// Gets the duration of the timeline.
    #[must_use]
    pub fn duration(&self) -> Duration {
        if self.tracks.is_empty() {
            return Duration::zero();
        }

        // Find the track that ends last
        self.tracks
            .iter()
            .map(|track| track.duration())
            .max()
            .unwrap_or_else(Duration::zero)
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}
