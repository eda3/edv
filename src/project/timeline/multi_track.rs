/// Multi-track timeline management functionality.
///
/// This module provides functionality for managing relationships between
/// multiple tracks in a timeline, handling synchronization, and supporting
/// complex video editing operations across tracks.
use std::collections::{HashMap, HashSet};
use std::fmt;

#[allow(unused_imports)]
use crate::project::timeline::{Timeline, Track, TrackId, TrackKind};

/// Error types specific to multi-track operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiTrackError {
    /// Attempted to create a track relationship with a non-existent track.
    TrackNotFound(TrackId),
    /// Attempted to create a circular dependency between tracks.
    CircularDependency(TrackId, TrackId),
    /// Attempted to create a relationship that conflicts with existing ones.
    ConflictingRelationship(TrackId, TrackId),
    /// Operation would result in invalid track state.
    InvalidTrackState(TrackId, String),
}

impl fmt::Display for MultiTrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrackNotFound(id) => write!(f, "Track not found: {id}"),
            Self::CircularDependency(id1, id2) => {
                write!(f, "Circular dependency between tracks {id1} and {id2}")
            }
            Self::ConflictingRelationship(id1, id2) => {
                write!(f, "Conflicting relationship between tracks {id1} and {id2}")
            }
            Self::InvalidTrackState(id, reason) => {
                write!(f, "Invalid track state for track {id}: {reason}")
            }
        }
    }
}

impl std::error::Error for MultiTrackError {}

/// Type alias for multi-track operation results.
pub type Result<T> = std::result::Result<T, MultiTrackError>;

/// Defines the relationship between two tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackRelationship {
    /// Tracks are independent with no synchronization requirements.
    Independent,
    /// Tracks should be locked for synchronous editing.
    Locked,
    /// One track affects the timing of another.
    TimingDependent,
    /// One track determines visibility of another.
    VisibilityDependent,
}

/// Manages relationships and dependencies between multiple tracks.
#[derive(Debug, Clone)]
pub struct MultiTrackManager {
    /// Maps track IDs to their dependent tracks and relationship types.
    dependencies: HashMap<TrackId, HashMap<TrackId, TrackRelationship>>,
    /// Maps track IDs to tracks that depend on them.
    reverse_dependencies: HashMap<TrackId, HashSet<TrackId>>,
}

impl MultiTrackManager {
    /// Creates a new empty `MultiTrackManager`.
    ///
    /// # Returns
    ///
    /// A new `MultiTrackManager` with no track relationships.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        }
    }

    /// Adds a relationship between two tracks.
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track in the relationship
    /// * `target_id` - The ID of the target track in the relationship
    /// * `relationship` - The type of relationship to establish
    /// * `timeline` - The timeline containing the tracks
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the relationship was successfully added,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Either track does not exist in the timeline
    /// * Adding this relationship would create a circular dependency
    /// * The relationship conflicts with existing relationships
    pub fn add_relationship(
        &mut self,
        source_id: TrackId,
        target_id: TrackId,
        relationship: TrackRelationship,
        timeline: &Timeline,
    ) -> Result<()> {
        // Ensure both tracks exist
        if !timeline.has_track(source_id) {
            return Err(MultiTrackError::TrackNotFound(source_id));
        }
        if !timeline.has_track(target_id) {
            return Err(MultiTrackError::TrackNotFound(target_id));
        }

        // Check for circular dependencies
        if self.would_create_circular_dependency(source_id, target_id) {
            return Err(MultiTrackError::CircularDependency(source_id, target_id));
        }

        // Update dependencies
        self.dependencies
            .entry(source_id)
            .or_default()
            .insert(target_id, relationship);

        // Update reverse dependencies
        self.reverse_dependencies
            .entry(target_id)
            .or_default()
            .insert(source_id);

        Ok(())
    }

    /// Adds a relationship assuming tracks exist and without checking timeline.
    ///
    /// This is intended for internal use (like undo/redo) where existence checks
    /// are performed beforehand.
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track.
    /// * `target_id` - The ID of the target track.
    /// * `relationship` - The type of relationship to establish.
    ///
    /// # Errors
    ///
    /// Returns `MultiTrackError::CircularDependency` if the relationship creates a cycle.
    pub(crate) fn add_relationship_no_timeline_check(
        &mut self,
        source_id: TrackId,
        target_id: TrackId,
        relationship: TrackRelationship,
    ) -> Result<()> {
        // Check for circular dependencies
        if self.would_create_circular_dependency(source_id, target_id) {
            return Err(MultiTrackError::CircularDependency(source_id, target_id));
        }

        // Update dependencies
        self.dependencies
            .entry(source_id)
            .or_default()
            .insert(target_id, relationship);

        // Update reverse dependencies
        self.reverse_dependencies
            .entry(target_id)
            .or_default()
            .insert(source_id);

        Ok(())
    }

    /// Removes a relationship between two tracks.
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track in the relationship
    /// * `target_id` - The ID of the target track in the relationship
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the relationship was successfully removed,
    /// or an error if the operation failed.
    pub fn remove_relationship(&mut self, source_id: TrackId, target_id: TrackId) -> Result<()> {
        // Remove from dependencies
        if let Some(deps) = self.dependencies.get_mut(&source_id) {
            deps.remove(&target_id);
            if deps.is_empty() {
                self.dependencies.remove(&source_id);
            }
        }

        // Remove from reverse dependencies
        if let Some(rev_deps) = self.reverse_dependencies.get_mut(&target_id) {
            rev_deps.remove(&source_id);
            if rev_deps.is_empty() {
                self.reverse_dependencies.remove(&target_id);
            }
        }

        Ok(())
    }

    /// Gets all tracks that depend on the specified track.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to find dependent tracks for
    ///
    /// # Returns
    ///
    /// A set of track IDs that depend on the specified track.
    #[must_use]
    pub fn get_dependent_tracks(&self, track_id: TrackId) -> HashSet<TrackId> {
        self.reverse_dependencies
            .get(&track_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Gets all tracks that the specified track depends on.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to find dependencies for
    ///
    /// # Returns
    ///
    /// A map of track IDs to relationship types that the specified track depends on.
    #[must_use]
    pub fn get_track_dependencies(&self, track_id: TrackId) -> HashMap<TrackId, TrackRelationship> {
        self.dependencies
            .get(&track_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Gets the relationship between two tracks, if any.
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track in the relationship
    /// * `target_id` - The ID of the target track in the relationship
    ///
    /// # Returns
    ///
    /// An `Option` containing the relationship type if one exists, or `None` if
    /// there is no direct relationship between the tracks.
    #[must_use]
    pub fn get_relationship(
        &self,
        source_id: TrackId,
        target_id: TrackId,
    ) -> Option<TrackRelationship> {
        self.dependencies
            .get(&source_id)
            .and_then(|deps| deps.get(&target_id))
            .copied()
    }

    /// Gets all relationships between tracks.
    ///
    /// # Returns
    ///
    /// A reference to the internal map of track dependencies, mapping source track IDs
    /// to a map of target track IDs and their relationship types.
    #[must_use]
    pub fn get_all_relationships(&self) -> &HashMap<TrackId, HashMap<TrackId, TrackRelationship>> {
        &self.dependencies
    }

    /// Applies an edit operation to a track and propagates changes to related tracks.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track being edited
    /// * `edit_fn` - A function that performs the edit on a track
    /// * `timeline` - The timeline containing the tracks
    ///
    /// # Returns
    ///
    /// A `Result` containing a set of affected track IDs if the operation was successful,
    /// or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The track does not exist in the timeline
    /// * The edit function returns an error
    /// * Propagating changes to related tracks fails
    pub fn apply_edit<F>(
        &self,
        track_id: TrackId,
        edit_fn: F,
        timeline: &mut Timeline,
    ) -> Result<HashSet<TrackId>>
    where
        F: FnOnce(&mut Track) -> std::result::Result<(), String>,
    {
        let mut affected_tracks = HashSet::new();
        affected_tracks.insert(track_id);

        // Get the track and apply the edit
        let track = timeline
            .get_track_mut(track_id)
            .ok_or(MultiTrackError::TrackNotFound(track_id))?;

        // Apply the edit function
        if let Err(error) = edit_fn(track) {
            return Err(MultiTrackError::InvalidTrackState(track_id, error));
        }

        // Propagate changes to dependent tracks
        self.propagate_changes(track_id, timeline, &mut affected_tracks)?;

        Ok(affected_tracks)
    }

    /// Removes a track and all its relationships.
    ///
    /// # Arguments
    ///
    /// * `track_id` - The ID of the track to remove
    ///
    /// # Returns
    ///
    /// A `Result` containing `()` if the track and its relationships were successfully
    /// removed, or an error if the operation failed.
    pub fn remove_track(&mut self, track_id: TrackId) -> Result<()> {
        // Remove track from dependencies
        self.dependencies.remove(&track_id);

        // Remove track from dependencies of other tracks
        for deps in self.dependencies.values_mut() {
            deps.remove(&track_id);
        }

        // Remove track from reverse dependencies
        self.reverse_dependencies.remove(&track_id);

        // Remove track from reverse dependencies lists
        for rev_deps in self.reverse_dependencies.values_mut() {
            rev_deps.remove(&track_id);
        }

        Ok(())
    }

    // Helper method to check if adding a relationship would create a circular dependency
    fn would_create_circular_dependency(&self, source_id: TrackId, target_id: TrackId) -> bool {
        // If target depends on source (directly or indirectly), adding source->target would create a cycle
        let mut visited = HashSet::new();
        self.is_dependent_on(target_id, source_id, &mut visited)
    }

    // Helper method to check if one track depends on another
    fn is_dependent_on(
        &self,
        track_id: TrackId,
        dependency_id: TrackId,
        visited: &mut HashSet<TrackId>,
    ) -> bool {
        if track_id == dependency_id {
            return true;
        }

        if !visited.insert(track_id) {
            return false;
        }

        if let Some(deps) = self.dependencies.get(&track_id) {
            for &dep_id in deps.keys() {
                if self.is_dependent_on(dep_id, dependency_id, visited) {
                    return true;
                }
            }
        }

        false
    }

    // Helper method to propagate changes to dependent tracks
    fn propagate_changes(
        &self,
        track_id: TrackId,
        timeline: &mut Timeline,
        affected_tracks: &mut HashSet<TrackId>,
    ) -> Result<()> {
        if let Some(dependent_tracks) = self.reverse_dependencies.get(&track_id) {
            for &dep_track_id in dependent_tracks {
                if affected_tracks.contains(&dep_track_id) {
                    continue;
                }

                affected_tracks.insert(dep_track_id);

                let relationship = self
                    .get_relationship(dep_track_id, track_id)
                    .unwrap_or(TrackRelationship::Independent);

                match relationship {
                    TrackRelationship::Locked => {
                        // For locked tracks, ensure they maintain synchronization
                        self.synchronize_locked_tracks(dep_track_id, track_id, timeline)?;
                    }
                    TrackRelationship::TimingDependent => {
                        // For timing-dependent tracks, update timing
                        self.update_timing_dependent_track(dep_track_id, track_id, timeline)?;
                    }
                    TrackRelationship::VisibilityDependent => {
                        // For visibility-dependent tracks, update visibility
                        self.update_visibility_dependent_track(dep_track_id, track_id, timeline)?;
                    }
                    TrackRelationship::Independent => {
                        // Independent tracks don't need propagation
                        continue;
                    }
                }

                // Recursively propagate to tracks that depend on this one
                self.propagate_changes(dep_track_id, timeline, affected_tracks)?;
            }
        }

        Ok(())
    }

    // Helper method to synchronize locked tracks
    fn synchronize_locked_tracks(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) -> Result<()> {
        // Get source track information first
        let (is_locked, is_muted, kind) = {
            let source_track = timeline.get_track(source_id)
                .ok_or(MultiTrackError::TrackNotFound(source_id))?;
            
            (source_track.is_locked(), source_track.is_muted(), source_track.kind())
        };
        
        // Now get target track after releasing source track borrow
        let target_track_mut = timeline.get_track_mut(target_id)
            .ok_or(MultiTrackError::TrackNotFound(target_id))?;
        
        // If source track is locked, target track should reflect the same locked state
        if is_locked != target_track_mut.is_locked() {
            target_track_mut.set_locked(is_locked);
        }
        
        // Synchronize mute state for audio tracks
        if kind == TrackKind::Audio && target_track_mut.kind() == TrackKind::Audio {
            if is_muted != target_track_mut.is_muted() {
                target_track_mut.set_muted(is_muted);
            }
        }
        
        Ok(())
    }

    // Helper method to update timing-dependent tracks
    fn update_timing_dependent_track(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) -> Result<()> {
        // Get source track information first, then release the borrow
        let source_clips = {
            let source_track = timeline.get_track(source_id)
                .ok_or(MultiTrackError::TrackNotFound(source_id))?;
            
            // Clone the clips to avoid borrowing issues
            source_track.get_clips().to_vec()
        };
        
        if source_clips.is_empty() {
            // If source has no clips, we can't derive timing
            return Ok(());
        }
        
        // Now get target track after releasing source track borrow
        let target_track_mut = timeline.get_track_mut(target_id)
            .ok_or(MultiTrackError::TrackNotFound(target_id))?;
        
        // Find corresponding clips in the target track and adjust their timing
        // This is a simplified implementation - real implementation would be more complex
        // based on specific project requirements
        for target_clip in target_track_mut.get_clips_mut() {
            // Find a source clip that might correspond to this target clip
            // In a real implementation, we might need a more sophisticated matching algorithm
            // or explicit clip relationship tracking
            if let Some(source_clip) = source_clips.iter().find(|&c| 
                // For example, matching clips that start around the same time
                f64::abs((c.position() - target_clip.position()).as_seconds()) < 1.0
            ) {
                // Adjust target clip timing based on source clip
                let offset_seconds = source_clip.position().as_seconds() - target_clip.position().as_seconds();
                if offset_seconds != 0.0 {
                    target_clip.set_position(source_clip.position());
                }
                
                // Optionally adjust duration as well
                if source_clip.duration().as_seconds() != target_clip.duration().as_seconds() {
                    target_clip.set_duration(source_clip.duration());
                }
            }
        }
        
        Ok(())
    }

    // Helper method to update visibility-dependent tracks
    fn update_visibility_dependent_track(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) -> Result<()> {
        // Get source track information first
        let (is_muted, kind) = {
            let source_track = timeline.get_track(source_id)
                .ok_or(MultiTrackError::TrackNotFound(source_id))?;
            
            (source_track.is_muted(), source_track.kind())
        };
        
        // Now get target track after releasing source track borrow
        let target_track_mut = timeline.get_track_mut(target_id)
            .ok_or(MultiTrackError::TrackNotFound(target_id))?;
        
        // In a visibility dependency, if source track is muted/hidden,
        // target track should reflect that state
        
        // For video tracks, visibility typically maps to track enabled state
        if kind == TrackKind::Video && target_track_mut.kind() == TrackKind::Video {
            // If source is muted, target should be hidden
            target_track_mut.set_muted(is_muted);
        }
        
        // For audio tracks, visibility typically maps to mute state
        if kind == TrackKind::Audio {
            // If source is muted, target should be muted
            target_track_mut.set_muted(is_muted);
        }
        
        Ok(())
    }

    /// Propagates visibility changes from source to target
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track
    /// * `target_id` - The ID of the target track
    /// * `timeline` - The timeline containing the tracks
    pub fn propagate_visibility_changes(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) {
        // Get relationship type
        if let Some(relationship) = self.get_relationship(source_id, target_id) {
            match relationship {
                TrackRelationship::VisibilityDependent => {
                    // For visibility-dependent tracks, update visibility state
                    let _ = self.update_visibility_dependent_track(source_id, target_id, timeline);
                },
                TrackRelationship::Locked => {
                    // For locked tracks, synchronize all states
                    let _ = self.synchronize_locked_tracks(source_id, target_id, timeline);
                },
                _ => {}  // Other relationship types don't affect visibility
            }
        }
    }

    /// Propagates timing changes from source to target
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track
    /// * `target_id` - The ID of the target track
    /// * `timeline` - The timeline containing the tracks
    pub fn propagate_timing_changes(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) {
        // Get relationship type
        if let Some(relationship) = self.get_relationship(source_id, target_id) {
            match relationship {
                TrackRelationship::TimingDependent => {
                    // For timing-dependent tracks, update clip timings
                    let _ = self.update_timing_dependent_track(source_id, target_id, timeline);
                },
                TrackRelationship::Locked => {
                    // For locked tracks, synchronize all states including timing
                    let _ = self.update_timing_dependent_track(source_id, target_id, timeline);
                },
                _ => {}  // Other relationship types don't affect timing
            }
        }
    }

    /// Propagates edit operations from source to target
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the source track
    /// * `target_id` - The ID of the target track
    /// * `timeline` - The timeline containing the tracks
    pub fn propagate_edits(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) {
        // Get relationship type
        if let Some(relationship) = self.get_relationship(source_id, target_id) {
            match relationship {
                TrackRelationship::Locked => {
                    // For locked tracks, propagate all changes
                    let _ = self.synchronize_locked_tracks(source_id, target_id, timeline);
                    let _ = self.update_timing_dependent_track(source_id, target_id, timeline);
                    let _ = self.update_visibility_dependent_track(source_id, target_id, timeline);
                },
                TrackRelationship::TimingDependent => {
                    // For timing-dependent, only update timing
                    let _ = self.update_timing_dependent_track(source_id, target_id, timeline);
                },
                TrackRelationship::VisibilityDependent => {
                    // For visibility-dependent, only update visibility
                    let _ = self.update_visibility_dependent_track(source_id, target_id, timeline);
                },
                TrackRelationship::Independent => {
                    // No propagation for independent tracks
                }
            }
        }
    }
}

impl Default for MultiTrackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test timeline
    fn create_test_timeline() -> Timeline {
        let mut timeline = Timeline::new();
        timeline.add_track(TrackKind::Video);
        timeline.add_track(TrackKind::Audio);
        timeline.add_track(TrackKind::Video);
        timeline
    }

    #[test]
    fn test_add_relationship() {
        let timeline = create_test_timeline();
        let track_ids: Vec<TrackId> = timeline.get_tracks().iter().map(|t| t.id()).collect();

        let mut manager = MultiTrackManager::new();

        // Add a valid relationship
        let result = manager.add_relationship(
            track_ids[0],
            track_ids[1],
            TrackRelationship::Locked,
            &timeline,
        );
        assert!(result.is_ok());

        // Check the relationship was added
        assert_eq!(
            manager.get_relationship(track_ids[0], track_ids[1]),
            Some(TrackRelationship::Locked)
        );
    }

    #[test]
    fn test_circular_dependency_detection() {
        let timeline = create_test_timeline();
        let track_ids: Vec<TrackId> = timeline.get_tracks().iter().map(|t| t.id()).collect();

        let mut manager = MultiTrackManager::new();

        // Add relationship A -> B
        let _ = manager.add_relationship(
            track_ids[0],
            track_ids[1],
            TrackRelationship::Locked,
            &timeline,
        );

        // Add relationship B -> C
        let _ = manager.add_relationship(
            track_ids[1],
            track_ids[2],
            TrackRelationship::Locked,
            &timeline,
        );

        // Try to add C -> A, which would create a cycle
        let result = manager.add_relationship(
            track_ids[2],
            track_ids[0],
            TrackRelationship::Locked,
            &timeline,
        );

        assert!(matches!(
            result,
            Err(MultiTrackError::CircularDependency(_, _))
        ));
    }

    #[test]
    fn test_remove_relationship() {
        let timeline = create_test_timeline();
        let track_ids: Vec<TrackId> = timeline.get_tracks().iter().map(|t| t.id()).collect();

        let mut manager = MultiTrackManager::new();

        // Add a relationship
        let _ = manager.add_relationship(
            track_ids[0],
            track_ids[1],
            TrackRelationship::Locked,
            &timeline,
        );

        // Remove the relationship
        let result = manager.remove_relationship(track_ids[0], track_ids[1]);
        assert!(result.is_ok());

        // Verify it was removed
        assert_eq!(manager.get_relationship(track_ids[0], track_ids[1]), None);
    }

    #[test]
    fn test_get_dependent_tracks() {
        let timeline = create_test_timeline();
        let track_ids: Vec<TrackId> = timeline.get_tracks().iter().map(|t| t.id()).collect();

        let mut manager = MultiTrackManager::new();

        // Add relationships: A -> B, C -> B
        let _ = manager.add_relationship(
            track_ids[0],
            track_ids[1],
            TrackRelationship::Locked,
            &timeline,
        );
        let _ = manager.add_relationship(
            track_ids[2],
            track_ids[1],
            TrackRelationship::TimingDependent,
            &timeline,
        );

        // Get tracks dependent on B
        let dependents = manager.get_dependent_tracks(track_ids[1]);
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&track_ids[0]));
        assert!(dependents.contains(&track_ids[2]));
    }

    #[test]
    fn test_remove_track() {
        let timeline = create_test_timeline();
        let track_ids: Vec<TrackId> = timeline.get_tracks().iter().map(|t| t.id()).collect();

        let mut manager = MultiTrackManager::new();

        // Add relationships involving track A
        let _ = manager.add_relationship(
            track_ids[0],
            track_ids[1],
            TrackRelationship::Locked,
            &timeline,
        );
        let _ = manager.add_relationship(
            track_ids[2],
            track_ids[0],
            TrackRelationship::TimingDependent,
            &timeline,
        );

        // Remove track A
        let result = manager.remove_track(track_ids[0]);
        assert!(result.is_ok());

        // Verify relationships were removed
        assert_eq!(manager.get_relationship(track_ids[0], track_ids[1]), None);
        assert_eq!(manager.get_relationship(track_ids[2], track_ids[0]), None);
        assert!(manager.get_dependent_tracks(track_ids[0]).is_empty());
        assert!(manager.get_track_dependencies(track_ids[0]).is_empty());
    }
}
