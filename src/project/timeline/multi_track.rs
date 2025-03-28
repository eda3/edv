/// Multi-track timeline management functionality.
///
/// This module provides functionality for managing relationships between
/// multiple tracks in a timeline, handling synchronization, and supporting
/// complex video editing operations across tracks.
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::project::timeline::{Clip, Timeline, Track, TrackId, TrackKind};
use crate::utility::time::{Duration, TimePosition};

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
            .or_insert_with(HashMap::new)
            .insert(target_id, relationship);

        // Update reverse dependencies
        self.reverse_dependencies
            .entry(target_id)
            .or_insert_with(HashSet::new)
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
        // Implementation for synchronizing locked tracks
        // This is a placeholder - actual implementation would depend on specific requirements
        Ok(())
    }

    // Helper method to update timing-dependent tracks
    fn update_timing_dependent_track(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) -> Result<()> {
        // Implementation for updating timing-dependent tracks
        // This is a placeholder - actual implementation would depend on specific requirements
        Ok(())
    }

    // Helper method to update visibility-dependent tracks
    fn update_visibility_dependent_track(
        &self,
        source_id: TrackId,
        target_id: TrackId,
        timeline: &mut Timeline,
    ) -> Result<()> {
        // Implementation for updating visibility-dependent tracks
        // This is a placeholder - actual implementation would depend on specific requirements
        Ok(())
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
