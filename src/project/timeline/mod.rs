use std::collections::HashMap;
/// Timeline management functionality.
///
/// This module provides functionality for creating, editing, and managing
/// video timelines, including tracks, clips, and multi-track relationships.
use uuid::Uuid;

pub mod history;
pub mod keyframes;
pub mod multi_track;

use crate::utility::time::{Duration, TimePosition};

// Export history types as well
pub use history::{
    EditAction, EditHistory, HistoryEntry, HistoryError, TransactionGroup, UndoableAction,
};

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

    /// Optional keyframe animation applied to this track
    keyframes: Option<keyframes::KeyframeAnimation>,
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
            keyframes: None,
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

    /// Gets mutable references to all clips in the track.
    pub fn get_clips_mut(&mut self) -> &mut [Clip] {
        &mut self.clips
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

    /// Gets the keyframe animation applied to this track.
    #[must_use]
    pub fn keyframes(&self) -> Option<&keyframes::KeyframeAnimation> {
        self.keyframes.as_ref()
    }

    /// Gets the keyframe animation applied to this track (mutable).
    #[must_use]
    pub fn keyframes_mut(&mut self) -> Option<&mut keyframes::KeyframeAnimation> {
        self.keyframes.as_mut()
    }

    /// Sets the keyframe animation for this track.
    pub fn set_keyframes(&mut self, keyframes: Option<keyframes::KeyframeAnimation>) {
        self.keyframes = keyframes;
    }

    /// Creates a new keyframe animation for this track if none exists.
    pub fn create_keyframes(&mut self) -> &mut keyframes::KeyframeAnimation {
        if self.keyframes.is_none() {
            // トラックの長さに基づいてキーフレームアニメーションの期間を設定
            let duration = self.duration();
            self.keyframes = Some(keyframes::KeyframeAnimation::new(duration));
        }
        self.keyframes.as_mut().unwrap()
    }
}

/// Timeline data structure.
#[derive(Debug, Clone)]
pub struct Timeline {
    /// Tracks in the timeline.
    tracks: Vec<Track>,

    /// Mapping from track ID to index in the tracks vector.
    track_index_map: HashMap<TrackId, usize>,

    /// Multi-track relationship manager.
    multi_track_manager: multi_track::MultiTrackManager,

    /// Edit history.
    history: history::EditHistory,
}

impl Timeline {
    /// Creates a new empty timeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            track_index_map: HashMap::new(),
            multi_track_manager: multi_track::MultiTrackManager::new(),
            history: history::EditHistory::new(),
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

    /// Finds the ID of the track containing the specified clip.
    ///
    /// # Arguments
    ///
    /// * `clip_id` - The ID of the clip to search for.
    ///
    /// # Returns
    ///
    /// An `Option<TrackId>` containing the ID of the track if found, otherwise `None`.
    #[must_use]
    pub fn find_track_containing_clip(&self, clip_id: crate::project::ClipId) -> Option<TrackId> {
        self.tracks.iter().find_map(|track| {
            if track.get_clip(clip_id).is_some() {
                Some(track.id())
            } else {
                None
            }
        })
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

    /// Adds a clip to a track and records the action in history.
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
    pub fn add_clip_with_history(&mut self, track_id: TrackId, clip: Clip) -> Result<()> {
        // クリップを複製
        let clip_clone = clip.clone();

        // クリップを追加
        let result = self.add_clip(track_id, clip);

        // 成功した場合のみ履歴に記録
        if result.is_ok() {
            let action = history::EditAction::AddClip {
                track_id,
                clip: clip_clone,
            };
            self.history.record(action);
        }

        result
    }

    /// Removes a clip from a track and records the action in history.
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
    pub fn remove_clip_with_history(
        &mut self,
        track_id: TrackId,
        clip_id: crate::project::ClipId,
    ) -> Result<()> {
        // First, get the clip and its position in the track
        let clip_info = {
            let track = self
                .get_track(track_id)
                .ok_or(TimelineError::TrackNotFound(track_id))?;

            let clip_index = track
                .get_clips()
                .iter()
                .position(|c| c.id() == clip_id)
                .ok_or(TimelineError::ClipNotFound {
                    track: track_id,
                    clip: clip_id,
                })?;

            (track.get_clips()[clip_index].clone(), clip_index)
        };

        let (clip, clip_index) = clip_info;

        // Remove the clip
        let result = self.remove_clip(track_id, clip_id);

        // Record the action in history if successful
        if result.is_ok() {
            let action = history::EditAction::RemoveClip {
                track_id,
                clip,
                original_index: clip_index,
            };
            self.history.record(action);
        }

        result
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

        // Find the track with the longest duration
        self.tracks
            .iter()
            .map(|track| track.duration())
            .max()
            .unwrap_or_else(Duration::zero)
    }

    /// Splits a clip at the specified position.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track containing the clip
    /// * `clip_id` - The ID of the clip to split
    /// * `position` - The position at which to split the clip
    ///
    /// # Returns
    ///
    /// The ID of the new clip created from the split, or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The track or clip is not found
    /// * The position is outside the clip
    /// * The clip cannot be split for some other reason
    pub fn split_clip(
        &mut self,
        track_id: TrackId,
        clip_id: crate::project::ClipId,
        position: TimePosition,
    ) -> Result<crate::project::ClipId> {
        // Get the track
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;

        // Find the clip
        let clip_index = track.clips.iter().position(|c| c.id() == clip_id).ok_or(
            TimelineError::ClipNotFound {
                track: track_id,
                clip: clip_id,
            },
        )?;

        // Check if position is within the clip
        let position_check = {
            let clip = &track.clips[clip_index];
            if position <= clip.position() || position >= clip.end_position() {
                return Err(TimelineError::InvalidOperation(format!(
                    "Split position {position} is outside clip bounds"
                )));
            }

            // Calculate durations and offsets
            let split_offset = position - clip.position();

            // First part duration = split_offset
            let first_part_duration = Duration::from_seconds(split_offset.as_seconds());

            // Second part duration = original duration - first part duration
            let second_part_duration = clip.duration() - first_part_duration;

            // Calculate source positions
            let source_offset_ratio =
                first_part_duration.as_seconds() / clip.duration().as_seconds();
            let source_split_point = clip.source_start()
                + Duration::from_seconds(
                    (clip.source_end().as_seconds() - clip.source_start().as_seconds())
                        * source_offset_ratio,
                );

            (
                first_part_duration,
                second_part_duration,
                source_split_point,
                clip.asset_id(),
                clip.source_start(),
                clip.source_end(),
            )
        };

        // Destructure the tuple
        let (
            first_part_duration,
            second_part_duration,
            source_split_point,
            asset_id,
            _source_start,
            source_end,
        ) = position_check;

        // Create the second (new) clip
        let new_clip_id = crate::project::ClipId::new();
        let new_clip = Clip::new(
            new_clip_id,
            asset_id,
            position,
            second_part_duration,
            source_split_point,
            source_end,
        );

        // Modify the original clip (first part)
        let clip = &mut track.clips[clip_index];
        clip.set_duration(first_part_duration);
        clip.set_source_end(source_split_point);

        // Add the new clip
        track.clips.push(new_clip);

        // Re-sort clips by position
        track
            .clips
            .sort_by(|a, b| a.position().partial_cmp(&b.position()).unwrap());

        Ok(new_clip_id)
    }

    /// Merges two adjacent clips.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track containing the clips
    /// * `first_clip_id` - The ID of the first clip
    /// * `second_clip_id` - The ID of the second clip
    ///
    /// # Returns
    ///
    /// `Ok(())` if the clips were merged successfully, or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The track or either clip is not found
    /// * The clips are not adjacent
    /// * The clips use different assets
    /// * The clips cannot be merged for some other reason
    pub fn merge_clips(
        &mut self,
        track_id: TrackId,
        first_clip_id: crate::project::ClipId,
        second_clip_id: crate::project::ClipId,
    ) -> Result<()> {
        // Get the track
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;

        // Find both clips
        let first_clip_index = track
            .clips
            .iter()
            .position(|c| c.id() == first_clip_id)
            .ok_or(TimelineError::ClipNotFound {
                track: track_id,
                clip: first_clip_id,
            })?;

        let second_clip_index = track
            .clips
            .iter()
            .position(|c| c.id() == second_clip_id)
            .ok_or(TimelineError::ClipNotFound {
                track: track_id,
                clip: second_clip_id,
            })?;

        // Ensure correct order (first clip should come before second clip)
        let (first_idx, second_idx) = if track.clips[first_clip_index].position()
            < track.clips[second_clip_index].position()
        {
            (first_clip_index, second_clip_index)
        } else {
            (second_clip_index, first_clip_index)
        };

        // Check if clips are adjacent and using the same asset
        // Store information needed for merging
        let merge_info = {
            let first_clip = &track.clips[first_idx];
            let second_clip = &track.clips[second_idx];

            if first_clip.end_position() != second_clip.position() {
                return Err(TimelineError::InvalidOperation(
                    "Clips are not adjacent".to_string(),
                ));
            }

            // Check if clips use the same asset
            if first_clip.asset_id() != second_clip.asset_id() {
                return Err(TimelineError::InvalidOperation(
                    "Cannot merge clips from different assets".to_string(),
                ));
            }

            // Calculate merged duration and collect source end from second clip
            (
                first_clip.duration() + second_clip.duration(),
                second_clip.source_end(),
            )
        };

        // Destructure the tuple of results from above
        let (merged_duration, second_source_end) = merge_info;

        // Update the first clip to span the combined duration
        let first_clip = &mut track.clips[first_idx];
        first_clip.set_duration(merged_duration);
        first_clip.set_source_end(second_source_end);

        // Remove the second clip
        // Note: If second_idx < first_idx, the removal would affect first_idx
        // But we ensured earlier that first_idx < second_idx
        track.clips.remove(second_idx);

        Ok(())
    }

    /// Moves a clip from one track to another.
    ///
    /// # Arguments
    ///
    /// * `source_track_id` - The ID of the track containing the clip
    /// * `target_track_id` - The ID of the track to move the clip to
    /// * `clip_id` - The ID of the clip to move
    /// * `new_position` - Optional new position for the clip (if None, keeps the original position)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the clip was moved successfully, or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The source track, target track, or clip is not found
    /// * The target track is of a different kind than the source track
    /// * Moving the clip would cause an overlap in the target track
    /// * The clip cannot be moved for some other reason
    pub fn move_clip_to_track(
        &mut self,
        source_track_id: TrackId,
        target_track_id: TrackId,
        clip_id: crate::project::ClipId,
        new_position: Option<TimePosition>,
    ) -> Result<()> {
        // Check if source and target tracks exist
        if !self.has_track(source_track_id) {
            return Err(TimelineError::TrackNotFound(source_track_id));
        }

        if !self.has_track(target_track_id) {
            return Err(TimelineError::TrackNotFound(target_track_id));
        }

        // Check if tracks are of the same kind
        let source_kind = self.get_track(source_track_id).unwrap().kind();
        let target_kind = self.get_track(target_track_id).unwrap().kind();

        if source_kind != target_kind {
            return Err(TimelineError::InvalidOperation(format!(
                "Cannot move clip between different track kinds: {source_kind} and {target_kind}"
            )));
        }

        // Find and remove clip from source track
        let source_track = self.get_track_mut(source_track_id).unwrap();
        let clip_index = source_track
            .clips
            .iter()
            .position(|c| c.id() == clip_id)
            .ok_or(TimelineError::ClipNotFound {
                track: source_track_id,
                clip: clip_id,
            })?;

        // Get position before removing and clone the clip
        let _original_position = source_track.clips[clip_index].position();
        let mut clip = source_track.clips[clip_index].clone();
        source_track.clips.remove(clip_index);

        // Update position if requested
        if let Some(position) = new_position {
            clip.set_position(position);
        }

        // Get the final position for error reporting
        let final_position = clip.position();

        // Add clip to target track
        let target_track = self.get_track_mut(target_track_id).unwrap();

        // Check for overlap in target track
        let has_overlap = target_track.get_clips().iter().any(|c| {
            c.id() != clip.id()
                && c.position() < clip.position() + clip.duration()
                && c.position() + c.duration() > clip.position()
        });

        if has_overlap {
            // Restore clip to source track
            self.get_track_mut(source_track_id)
                .unwrap()
                .clips
                .push(clip);
            return Err(TimelineError::ClipOverlap {
                position: final_position,
            });
        }

        // Add clip to target track
        target_track.clips.push(clip);
        target_track
            .clips
            .sort_by(|a, b| a.position().partial_cmp(&b.position()).unwrap());

        Ok(())
    }

    /// Moves a clip to a different track and records the action in history.
    ///
    /// # Arguments
    ///
    /// * `source_track_id` - The ID of the track containing the clip
    /// * `target_track_id` - The ID of the track to move the clip to
    /// * `clip_id` - The ID of the clip to move
    /// * `new_position` - The new position for the clip, or None to keep the current position
    ///
    /// # Returns
    ///
    /// `Ok(())` if the clip was moved successfully, or an error if the source track,
    /// target track, or clip does not exist, or if the new position would cause overlap.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The source track does not exist
    /// * The target track does not exist
    /// * The clip does not exist in the source track
    /// * The new position would cause the clip to overlap with existing clips
    pub fn move_clip_to_track_with_history(
        &mut self,
        source_track_id: TrackId,
        target_track_id: TrackId,
        clip_id: crate::project::ClipId,
        new_position: Option<TimePosition>,
    ) -> Result<()> {
        // First, get the clip and its position
        let clip_info = {
            let source_track = self
                .get_track(source_track_id)
                .ok_or(TimelineError::TrackNotFound(source_track_id))?;

            let clip_index = source_track
                .get_clips()
                .iter()
                .position(|c| c.id() == clip_id)
                .ok_or(TimelineError::ClipNotFound {
                    track: source_track_id,
                    clip: clip_id,
                })?;

            let clip = source_track.get_clips()[clip_index].clone();
            let original_position = clip.position();

            (clip_index, original_position)
        };

        let (clip_index, original_position) = clip_info;

        // Move the clip
        let result =
            self.move_clip_to_track(source_track_id, target_track_id, clip_id, new_position);

        // Get the new position if the move was successful
        if result.is_ok() {
            let final_position = {
                let target_track = self
                    .get_track(target_track_id)
                    .ok_or(TimelineError::TrackNotFound(target_track_id))?;

                let moved_clip =
                    target_track
                        .get_clip(clip_id)
                        .ok_or(TimelineError::ClipNotFound {
                            track: target_track_id,
                            clip: clip_id,
                        })?;

                moved_clip.position()
            };

            // Record the action in history
            let action = history::EditAction::MoveClip {
                clip_id,
                original_track_id: source_track_id,
                original_position,
                original_index: clip_index,
                new_track_id: target_track_id,
                new_position: final_position,
            };
            self.history.record(action);
        }

        result
    }

    /// Gets a reference to the edit history.
    #[must_use]
    pub fn history(&self) -> &history::EditHistory {
        &self.history
    }

    /// Gets a mutable reference to the edit history.
    pub fn history_mut(&mut self) -> &mut history::EditHistory {
        &mut self.history
    }

    /// Undoes the last edit action.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the undo was successful, or an error if there is nothing to undo
    /// or if the undo operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * There is nothing to undo
    /// * The undo operation failed
    pub fn undo(&mut self) -> std::result::Result<(), history::HistoryError> {
        // 履歴に元に戻せるアクションがあるか確認
        if !self.history.can_undo() {
            return Err(history::HistoryError::NothingToUndo);
        }

        // 借用の問題を回避するため、一旦historyをクローンして取り出す
        let mut history_clone = self.history.clone();
        let last_entry_opt = history_clone.peek_undo().cloned();

        // 最後のエントリが取得できた場合、アクションを適用
        if let Some(entry) = last_entry_opt {
            // undo操作を実行
            let result = match entry {
                history::HistoryEntry::Single(action) => action.undo(self),
                history::HistoryEntry::Group(group) => {
                    // グループの場合は全アクションを逆順に適用
                    for action in group.actions().iter().rev() {
                        action.undo(self)?;
                    }
                    Ok(())
                }
            };

            // 操作が成功した場合は、履歴スタックを更新
            if result.is_ok() {
                self.history.shift_to_redo();
                Ok(())
            } else {
                // エラーを返す
                Err(history::HistoryError::ApplyActionError(
                    result.err().unwrap().to_string(),
                ))
            }
        } else {
            Err(history::HistoryError::NothingToUndo)
        }
    }

    /// Redoes the last undone edit action.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the redo was successful, or an error if there is nothing to redo
    /// or if the redo operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * There is nothing to redo
    /// * The redo operation failed
    pub fn redo(&mut self) -> std::result::Result<(), history::HistoryError> {
        // 履歴にやり直せるアクションがあるか確認
        if !self.history.can_redo() {
            return Err(history::HistoryError::NothingToRedo);
        }

        // 借用の問題を回避するため、一旦historyをクローンして取り出す
        let mut history_clone = self.history.clone();
        let last_entry_opt = history_clone.peek_redo().cloned();

        // 最後のエントリが取得できた場合、アクションを適用
        if let Some(entry) = last_entry_opt {
            // redo操作を実行
            let result = match entry {
                history::HistoryEntry::Single(action) => action.apply(self),
                history::HistoryEntry::Group(group) => {
                    // グループの場合は全アクションを順に適用
                    for action in group.actions() {
                        action.apply(self)?;
                    }
                    Ok(())
                }
            };

            // 操作が成功した場合は、履歴スタックを更新
            if result.is_ok() {
                self.history.shift_to_undo();
                Ok(())
            } else {
                // エラーを返す
                Err(history::HistoryError::ApplyActionError(
                    result.err().unwrap().to_string(),
                ))
            }
        } else {
            Err(history::HistoryError::NothingToRedo)
        }
    }

    /// Checks if there are any actions that can be undone.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Checks if there are any actions that can be redone.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Begins a transaction for grouping multiple edit actions.
    ///
    /// # Arguments
    ///
    /// * `description` - An optional description for the transaction
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transaction was started successfully, or an error if
    /// a transaction is already in progress.
    ///
    /// # Errors
    ///
    /// Returns an error if a transaction is already in progress.
    pub fn begin_transaction(
        &mut self,
        description: Option<String>,
    ) -> std::result::Result<(), history::HistoryError> {
        self.history.begin_transaction(description)
    }

    /// Commits the current transaction.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transaction was committed successfully, or an error if
    /// no transaction is in progress.
    ///
    /// # Errors
    ///
    /// Returns an error if no transaction is in progress.
    pub fn commit_transaction(&mut self) -> std::result::Result<(), history::HistoryError> {
        self.history.commit_transaction()
    }

    /// Rolls back the current transaction.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transaction was rolled back successfully, or an error if
    /// no transaction is in progress.
    ///
    /// # Errors
    ///
    /// Returns an error if no transaction is in progress.
    pub fn rollback_transaction(&mut self) -> std::result::Result<(), history::HistoryError> {
        self.history.rollback_transaction()
    }

    /// Adds a keyframe to a track for a specific property.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to add the keyframe to
    /// * `property` - The property name to animate
    /// * `time` - The time position of the keyframe
    /// * `value` - The value at the keyframe
    /// * `easing` - The easing function to use
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the keyframe was successfully added,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the track doesn't exist or the keyframe couldn't be added.
    pub fn add_keyframe(
        &mut self,
        track_id: TrackId,
        property: &str,
        time: crate::utility::time::TimePosition,
        value: f64,
        easing: keyframes::EasingFunction,
    ) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;

        // トラックにキーフレームアニメーションがなければ作成
        let keyframes = track.create_keyframes();

        // キーフレームを追加
        keyframes
            .add_keyframe(property, time, value, easing)
            .map_err(|e| TimelineError::InvalidOperation(format!("キーフレーム追加エラー: {}", e)))
    }

    /// Adds a keyframe to a track for a specific property and records the action in history.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to add the keyframe to
    /// * `property` - The property name to animate
    /// * `time` - The time position of the keyframe
    /// * `value` - The value at the keyframe
    /// * `easing` - The easing function to use
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the keyframe was successfully added,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the track doesn't exist or the keyframe couldn't be added.
    pub fn add_keyframe_with_history(
        &mut self,
        track_id: TrackId,
        property: &str,
        time: crate::utility::time::TimePosition,
        value: f64,
        easing: keyframes::EasingFunction,
    ) -> Result<()> {
        // プロパティ名をコピー
        let property_string = property.to_string();

        // アクションを実行
        let result = self.add_keyframe(track_id, property, time, value, easing);

        // 成功した場合のみ履歴に記録
        if result.is_ok() {
            let action = history::EditAction::AddKeyframe {
                track_id,
                property: property_string,
                time,
                value,
                easing,
            };

            self.history.record(action);
        }

        result
    }

    /// Gets the animated property value at a specific time.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track
    /// * `property` - The property name
    /// * `time` - The time position to get the value at
    ///
    /// # Returns
    ///
    /// An `Option` containing the interpolated value at the given time, or `None`
    /// if the track doesn't exist, doesn't have keyframe animation, or the property
    /// doesn't have keyframes.
    #[must_use]
    pub fn get_keyframe_value_at(
        &self,
        track_id: TrackId,
        property: &str,
        time: crate::utility::time::TimePosition,
    ) -> Option<f64> {
        self.get_track(track_id)
            .and_then(|track| track.keyframes())
            .and_then(|keyframes| keyframes.get_value_at(property, time))
    }

    /// Removes a keyframe from a track.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track
    /// * `property` - The property name
    /// * `time` - The time position of the keyframe to remove
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the keyframe was successfully removed,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the track doesn't exist, the track doesn't have keyframe
    /// animation, or the keyframe couldn't be found.
    pub fn remove_keyframe(
        &mut self,
        track_id: TrackId,
        property: &str,
        time: crate::utility::time::TimePosition,
    ) -> Result<()> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;

        if let Some(keyframes) = track.keyframes_mut() {
            if let Some(track) = keyframes.get_track_mut(property) {
                track.remove_keyframe(time).map_err(|e| {
                    TimelineError::InvalidOperation(format!("キーフレーム削除エラー: {}", e))
                })
            } else {
                Err(TimelineError::InvalidOperation(format!(
                    "プロパティが存在しません: {}",
                    property
                )))
            }
        } else {
            Err(TimelineError::InvalidOperation(
                "トラックにキーフレームアニメーションがありません".to_string(),
            ))
        }
    }

    /// Removes a keyframe from a track and records the action in history.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track
    /// * `property` - The property name
    /// * `time` - The time position of the keyframe to remove
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the keyframe was successfully removed,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the track doesn't exist, the track doesn't have keyframe
    /// animation, or the keyframe couldn't be found.
    pub fn remove_keyframe_with_history(
        &mut self,
        track_id: TrackId,
        property: &str,
        time: crate::utility::time::TimePosition,
    ) -> Result<()> {
        // 削除前にキーフレームの情報を保存
        let property_string = property.to_string();
        let value_and_easing = {
            self.get_track(track_id)
                .and_then(|track| track.keyframes())
                .and_then(|keyframes| keyframes.get_track(property))
                .and_then(|track| track.keyframes().iter().find(|kf| kf.time() == time))
                .map(|kf| (kf.value(), kf.easing()))
        };

        if let Some((value, easing)) = value_and_easing {
            // アクションを実行
            let result = self.remove_keyframe(track_id, property, time);

            // 成功した場合のみ履歴に記録
            if result.is_ok() {
                let action = history::EditAction::RemoveKeyframe {
                    track_id,
                    property: property_string,
                    time,
                    value,
                    easing,
                };

                self.history.record(action);
            }

            result
        } else {
            Err(TimelineError::InvalidOperation(
                "キーフレームが見つかりません".to_string(),
            ))
        }
    }

    /// Updates a keyframe on a track and records the action in history.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track
    /// * `property` - The property name
    /// * `time` - The time position of the keyframe
    /// * `new_value` - The new value for the keyframe
    /// * `new_easing` - The new easing function for the keyframe
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the keyframe was successfully updated,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the track doesn't exist, the track doesn't have keyframe
    /// animation, or the keyframe couldn't be found.
    pub fn update_keyframe_with_history(
        &mut self,
        track_id: TrackId,
        property: &str,
        time: crate::utility::time::TimePosition,
        new_value: f64,
        new_easing: keyframes::EasingFunction,
    ) -> Result<()> {
        // プロパティ名をコピー
        let property_string = property.to_string();

        // 更新前の値を取得
        let original_value_and_easing = {
            self.get_track(track_id)
                .and_then(|track| track.keyframes())
                .and_then(|keyframes| keyframes.get_track(property))
                .and_then(|track| track.keyframes().iter().find(|kf| kf.time() == time))
                .map(|kf| (kf.value(), kf.easing()))
        };

        if let Some((original_value, original_easing)) = original_value_and_easing {
            // キーフレームを削除して新しい値で追加
            let result = {
                self.remove_keyframe(track_id, property, time)
                    .and_then(|_| {
                        self.add_keyframe(track_id, property, time, new_value, new_easing)
                    })
            };

            // 成功した場合のみ履歴に記録
            if result.is_ok() {
                let action = history::EditAction::UpdateKeyframe {
                    track_id,
                    property: property_string,
                    time,
                    original_value,
                    new_value,
                    original_easing,
                    new_easing,
                };

                self.history.record(action);
            }

            result
        } else {
            Err(TimelineError::InvalidOperation(
                "キーフレームが見つかりません".to_string(),
            ))
        }
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{AssetId, ClipId};
    use crate::utility::time::{Duration, TimePosition};

    #[test]
    fn test_add_track() {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        assert!(timeline.has_track(track_id));
        assert_eq!(timeline.get_tracks().len(), 1);

        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(track.kind(), TrackKind::Video);
    }

    #[test]
    fn test_add_clip() {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        let asset_id = AssetId::new();
        let clip_id = ClipId::new();
        let clip = Clip::new(
            clip_id,
            asset_id,
            TimePosition::from_seconds(10.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        );

        let result = timeline.add_clip(track_id, clip);
        assert!(result.is_ok());

        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(track.get_clips().len(), 1);

        let clip = track.get_clip(clip_id).unwrap();
        assert_eq!(clip.position(), TimePosition::from_seconds(10.0));
        assert_eq!(clip.duration(), Duration::from_seconds(5.0));
    }

    #[test]
    fn test_clip_overlap() {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        // Add first clip at position 10.0 with duration 5.0
        let asset_id = AssetId::new();
        let clip1 = Clip::new(
            ClipId::new(),
            asset_id,
            TimePosition::from_seconds(10.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        );

        let result = timeline.add_clip(track_id, clip1);
        assert!(result.is_ok());

        // Try to add overlapping clip at position 12.0
        let clip2 = Clip::new(
            ClipId::new(),
            asset_id,
            TimePosition::from_seconds(12.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        );

        let result = timeline.add_clip(track_id, clip2);
        assert!(result.is_err());

        // Add non-overlapping clip
        let clip3 = Clip::new(
            ClipId::new(),
            asset_id,
            TimePosition::from_seconds(20.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        );

        let result = timeline.add_clip(track_id, clip3);
        assert!(result.is_ok());

        // Verify two clips are in the track
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(track.get_clips().len(), 2);
    }

    #[test]
    fn test_split_clip() {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        // Add a clip at position 10.0 with duration 10.0
        let asset_id = AssetId::new();
        let clip_id = ClipId::new();
        let clip = Clip::new(
            clip_id,
            asset_id,
            TimePosition::from_seconds(10.0),
            Duration::from_seconds(10.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(10.0),
        );

        let result = timeline.add_clip(track_id, clip);
        assert!(result.is_ok());

        // Split the clip at position 15.0
        let result = timeline.split_clip(track_id, clip_id, TimePosition::from_seconds(15.0));

        assert!(result.is_ok());
        let new_clip_id = result.unwrap();

        // Verify the split
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(track.get_clips().len(), 2);

        // Check original clip
        let original_clip = track.get_clip(clip_id).unwrap();
        assert_eq!(original_clip.position(), TimePosition::from_seconds(10.0));
        assert_eq!(original_clip.duration(), Duration::from_seconds(5.0));
        assert_eq!(original_clip.source_end(), TimePosition::from_seconds(5.0));

        // Check new clip
        let new_clip = track.get_clip(new_clip_id).unwrap();
        assert_eq!(new_clip.position(), TimePosition::from_seconds(15.0));
        assert_eq!(new_clip.duration(), Duration::from_seconds(5.0));
        assert_eq!(new_clip.source_start(), TimePosition::from_seconds(5.0));
        assert_eq!(new_clip.source_end(), TimePosition::from_seconds(10.0));
    }

    #[test]
    fn test_merge_clips() {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(TrackKind::Video);

        // Add two adjacent clips
        let asset_id = AssetId::new();
        let clip1_id = ClipId::new();
        let clip1 = Clip::new(
            clip1_id,
            asset_id,
            TimePosition::from_seconds(10.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        );

        let clip2_id = ClipId::new();
        let clip2 = Clip::new(
            clip2_id,
            asset_id,
            TimePosition::from_seconds(15.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(5.0),
            TimePosition::from_seconds(10.0),
        );

        timeline.add_clip(track_id, clip1).unwrap();
        timeline.add_clip(track_id, clip2).unwrap();

        // Merge the clips
        let result = timeline.merge_clips(track_id, clip1_id, clip2_id);
        assert!(result.is_ok());

        // Verify the merge
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(track.get_clips().len(), 1);

        // Check merged clip
        let merged_clip = track.get_clip(clip1_id).unwrap();
        assert_eq!(merged_clip.position(), TimePosition::from_seconds(10.0));
        assert_eq!(merged_clip.duration(), Duration::from_seconds(10.0));
        assert_eq!(merged_clip.source_start(), TimePosition::from_seconds(0.0));
        assert_eq!(merged_clip.source_end(), TimePosition::from_seconds(10.0));
    }

    #[test]
    fn test_move_clip_to_track() {
        let mut timeline = Timeline::new();
        let track1_id = timeline.add_track(TrackKind::Video);
        let track2_id = timeline.add_track(TrackKind::Video);

        // Add a clip to track1
        let asset_id = AssetId::new();
        let clip_id = ClipId::new();
        let clip = Clip::new(
            clip_id,
            asset_id,
            TimePosition::from_seconds(10.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        );

        timeline.add_clip(track1_id, clip).unwrap();

        // Move clip to track2
        let result = timeline.move_clip_to_track(track1_id, track2_id, clip_id, None);

        assert!(result.is_ok());

        // Verify the move
        let track1 = timeline.get_track(track1_id).unwrap();
        assert_eq!(track1.get_clips().len(), 0);

        let track2 = timeline.get_track(track2_id).unwrap();
        assert_eq!(track2.get_clips().len(), 1);

        // Check clip in new track
        let moved_clip = track2.get_clip(clip_id).unwrap();
        assert_eq!(moved_clip.position(), TimePosition::from_seconds(10.0));
    }
}
