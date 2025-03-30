use crate::project::ClipId;
/// Timeline editing history and undo/redo functionality.
///
/// This module provides the structures and logic for managing
/// an undo/redo history for timeline operations, including support
/// for grouping actions into transactions.
use crate::project::timeline::{Clip, TimelineError, Track, TrackId};
use crate::utility::time::{Duration, TimePosition};
// Import Timeline struct itself
use super::Timeline;
// Fix: Import the correct enum TrackRelationship
use crate::project::timeline::multi_track::TrackRelationship;

/// Represents a single, reversible action performed on the timeline.
///
/// Each variant stores the necessary information to undo the action.
#[derive(Debug, Clone)]
pub enum EditAction {
    /// Added a clip to a track.
    AddClip {
        track_id: TrackId,
        clip: Clip, // Store the whole clip to remove it on undo.
    },
    /// Removed a clip from a track.
    RemoveClip {
        track_id: TrackId,
        clip: Clip,            // Store the whole clip to re-add it on undo.
        original_index: usize, // Store original position for correct re-insertion
    },
    /// Moved a clip within a track or between tracks.
    MoveClip {
        clip_id: ClipId,
        original_track_id: TrackId,
        original_position: TimePosition,
        original_index: usize, // Index in the original track
        new_track_id: TrackId,
        new_position: TimePosition,
    },
    /// Changed the duration of a clip (e.g., trimming).
    SetClipDuration {
        clip_id: ClipId,
        track_id: TrackId, // To find the clip easily
        original_duration: Duration,
        new_duration: Duration,
        // We might need original source_start/end as well depending on implementation
        original_source_end: TimePosition,
        new_source_end: TimePosition,
    },
    /// Changed the position of a clip.
    SetClipPosition {
        clip_id: ClipId,
        track_id: TrackId,
        original_position: TimePosition,
        new_position: TimePosition,
    },
    /// Added a new track.
    AddTrack {
        track_id: TrackId,
        // Store enough info to reconstruct, or maybe the whole Track? Let's start simple.
        track_kind: super::TrackKind, // Use super:: to refer to parent module type
        track_name: String,
    },
    /// Removed a track.
    RemoveTrack {
        // Store the whole track data to restore it on undo
        track_data: Track,
        original_index: usize, // Index in the timeline's track list
                               // TODO: Consider storing relationships if they aren't automatically handled
    },
    // Added Track property changes
    SetTrackName {
        track_id: TrackId,
        original_name: String,
        new_name: String,
    },
    SetTrackMuted {
        track_id: TrackId,
        original_muted: bool,
        new_muted: bool,
    },
    SetTrackLocked {
        track_id: TrackId,
        original_locked: bool,
        new_locked: bool,
    },
    // TODO: Add actions for TrackRelationship changes
    // TODO: Add actions for SplitClip, MergeClips
    AddRelationship {
        source_id: TrackId,
        target_id: TrackId,
        relationship_kind: TrackRelationship, // Store the enum variant
    },
    RemoveRelationship {
        source_id: TrackId,
        target_id: TrackId,
        original_relationship_kind: TrackRelationship, // Store the original enum variant
    },
    UpdateRelationship {
        source_id: TrackId,
        target_id: TrackId,
        original_relationship_kind: TrackRelationship,
        updated_relationship_kind: TrackRelationship,
    },
}

/// Defines methods for applying and undoing timeline actions.
///
/// This trait is implemented by `EditAction` and potentially `TransactionGroup`.
pub trait UndoableAction {
    /// Applies the action to the timeline.
    ///
    /// # Arguments
    ///
    /// * `timeline` - A mutable reference to the `Timeline` to modify.
    ///
    /// # Errors
    ///
    /// Returns a `TimelineError` if applying the action fails.
    fn apply(&self, timeline: &mut Timeline) -> Result<(), TimelineError>;

    /// Undoes the action on the timeline.
    ///
    /// # Arguments
    ///
    /// * `timeline` - A mutable reference to the `Timeline` to revert.
    ///
    /// # Errors
    ///
    /// Returns a `TimelineError` if undoing the action fails.
    fn undo(&self, timeline: &mut Timeline) -> Result<(), TimelineError>;
}

impl UndoableAction for EditAction {
    /// Applies the specific `EditAction` variant to the timeline.
    ///
    /// # Errors
    ///
    /// Propagates errors from the underlying timeline operations.
    fn apply(&self, timeline: &mut Timeline) -> Result<(), TimelineError> {
        match self {
            EditAction::AddClip { track_id, clip } => {
                // Simply add the clip (we assume it was valid when recorded)
                timeline.add_clip(*track_id, clip.clone())
            }
            EditAction::RemoveClip { track_id, clip, .. } => {
                // Remove the clip by ID
                timeline.remove_clip(*track_id, clip.id())
            }
            EditAction::MoveClip {
                clip_id,
                new_track_id,
                new_position,
                ..
            } => {
                // Need to find the original track first to move *from*.
                // This requires a more complex lookup or modification of Timeline::move_clip_to_track.
                // For now, assume we can find the clip and move it directly.
                // Find the clip across all tracks first.
                let source_track_id =
                    timeline
                        .find_track_containing_clip(*clip_id)
                        .ok_or_else(|| TimelineError::ClipNotFound {
                            track: TrackId::new(),
                            clip: *clip_id,
                        })?; // Dummy TrackId for error

                timeline.move_clip_to_track(
                    source_track_id,
                    *new_track_id,
                    *clip_id,
                    Some(*new_position),
                )
            }
            EditAction::SetClipDuration {
                clip_id,
                track_id,
                new_duration,
                new_source_end,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                let clip =
                    track
                        .get_clip_mut(*clip_id)
                        .ok_or_else(|| TimelineError::ClipNotFound {
                            track: *track_id,
                            clip: *clip_id,
                        })?;
                clip.set_duration(*new_duration);
                clip.set_source_end(*new_source_end);
                // TODO: Potentially re-sort or validate track?
                Ok(())
            }
            EditAction::SetClipPosition {
                clip_id,
                track_id,
                new_position,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                let clip =
                    track
                        .get_clip_mut(*clip_id)
                        .ok_or_else(|| TimelineError::ClipNotFound {
                            track: *track_id,
                            clip: *clip_id,
                        })?;
                clip.set_position(*new_position);
                // Re-sort clips within the track after changing position
                track
                    .get_clips_mut()
                    .sort_by(|a, b| a.position().partial_cmp(&b.position()).unwrap());
                Ok(())
            }
            EditAction::AddTrack {
                track_id,
                track_kind,
                track_name,
            } => {
                // Create and add the track. We assume ID collision is negligible.
                // A more robust approach might check ID existence or use a different way to add.
                let mut new_track = Track::new(*track_id, *track_kind);
                new_track.set_name(track_name);
                // Insert the track at the original index if possible, otherwise append.
                // Need the original index here. Let's assume append for now.
                timeline.tracks.push(new_track); // Direct access, consider a method
                Ok(())
            }
            EditAction::RemoveTrack { track_data, .. } => timeline.remove_track(track_data.id()),
            EditAction::SetTrackName {
                track_id, new_name, ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.set_name(new_name);
                Ok(())
            }
            EditAction::SetTrackMuted {
                track_id,
                new_muted,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.set_muted(*new_muted);
                Ok(())
            }
            EditAction::SetTrackLocked {
                track_id,
                new_locked,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.set_locked(*new_locked);
                Ok(())
            }
            EditAction::AddRelationship { source_id, target_id, relationship_kind } => {
                timeline.multi_track_manager_mut().add_relationship_no_timeline_check(
                    *source_id,
                    *target_id,
                    *relationship_kind, // Pass the enum variant
                )?;
                Ok(())
            }
            EditAction::RemoveRelationship { source_id, target_id, .. } => {
                timeline.multi_track_manager_mut().remove_relationship(
                    *source_id,
                    *target_id,
                )?;
                Ok(())
            }
            EditAction::UpdateRelationship {
                source_id,
                target_id,
                updated_relationship_kind,
                 .. // Use .. to ignore original_relationship_kind if not used
            } => {
                let _ = timeline.multi_track_manager_mut().remove_relationship(*source_id, *target_id);
                timeline.multi_track_manager_mut().add_relationship_no_timeline_check(
                    *source_id,
                    *target_id,
                    *updated_relationship_kind, // Pass the new enum variant
                )?;
                Ok(())
            }
        }
    }

    /// Undoes the specific `EditAction` variant on the timeline.
    ///
    /// # Errors
    ///
    /// Propagates errors from the underlying timeline operations.
    fn undo(&self, timeline: &mut Timeline) -> Result<(), TimelineError> {
        match self {
            EditAction::AddClip { track_id, clip } => {
                // Undo adding a clip by removing it
                timeline.remove_clip(*track_id, clip.id())
            }
            EditAction::RemoveClip {
                track_id,
                clip,
                original_index,
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                let insert_index = (*original_index).min(track.clips.len());
                track.clips.insert(insert_index, clip.clone());
                Ok(())
            }
            EditAction::MoveClip {
                clip_id,
                original_track_id,
                original_position,
                new_track_id,
                ..
            } => {
                // Undo moving by moving it back
                timeline.move_clip_to_track(
                    *new_track_id,
                    *original_track_id,
                    *clip_id,
                    Some(*original_position),
                )
            }
            EditAction::SetClipDuration {
                clip_id,
                track_id,
                original_duration,
                original_source_end,
                ..
            } => {
                // Undo setting duration by setting it back to original
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                let clip =
                    track
                        .get_clip_mut(*clip_id)
                        .ok_or_else(|| TimelineError::ClipNotFound {
                            track: *track_id,
                            clip: *clip_id,
                        })?;
                clip.set_duration(*original_duration);
                clip.set_source_end(*original_source_end);
                // TODO: Potentially re-sort or validate track?
                Ok(())
            }
            EditAction::SetClipPosition {
                clip_id,
                track_id,
                original_position,
                ..
            } => {
                // Undo setting position by setting it back to original
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                let clip =
                    track
                        .get_clip_mut(*clip_id)
                        .ok_or_else(|| TimelineError::ClipNotFound {
                            track: *track_id,
                            clip: *clip_id,
                        })?;
                clip.set_position(*original_position);
                // Re-sort clips within the track after changing position
                track
                    .get_clips_mut()
                    .sort_by(|a, b| a.position().partial_cmp(&b.position()).unwrap());
                Ok(())
            }
            EditAction::AddTrack { track_id, .. } => {
                // Undo adding a track by removing it
                timeline.remove_track(*track_id)
            }
            EditAction::RemoveTrack {
                track_data,
                original_index,
            } => {
                let insert_index = (*original_index).min(timeline.tracks.len());
                timeline.tracks.insert(insert_index, track_data.clone());
                Ok(())
            }
            EditAction::SetTrackName {
                track_id,
                original_name,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.set_name(original_name);
                Ok(())
            }
            EditAction::SetTrackMuted {
                track_id,
                original_muted,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.set_muted(*original_muted);
                Ok(())
            }
            EditAction::SetTrackLocked {
                track_id,
                original_locked,
                ..
            } => {
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.set_locked(*original_locked);
                Ok(())
            }
            EditAction::AddRelationship { source_id, target_id, .. } => {
                timeline.multi_track_manager_mut().remove_relationship(
                    *source_id,
                    *target_id,
                )?;
                Ok(())
            }
            EditAction::RemoveRelationship { source_id, target_id, original_relationship_kind } => {
                timeline.multi_track_manager_mut().add_relationship_no_timeline_check(
                    *source_id,
                    *target_id,
                    *original_relationship_kind, // Pass the original enum variant
                )?;
                Ok(())
            }
            EditAction::UpdateRelationship {
                source_id,
                target_id,
                original_relationship_kind,
                .. // Use .. to ignore updated_relationship_kind if not used
            } => {
                let _ = timeline.multi_track_manager_mut().remove_relationship(*source_id, *target_id);
                timeline.multi_track_manager_mut().add_relationship_no_timeline_check(
                    *source_id,
                    *target_id,
                    *original_relationship_kind, // Pass the original enum variant
                )?;
                Ok(())
            }
        }
    }
}

/// Represents a group of actions that should be undone/redone together.
#[derive(Debug, Clone)]
pub struct TransactionGroup {
    /// A descriptive name for the transaction (optional).
    description: Option<String>,
    /// The sequence of actions within this transaction.
    actions: Vec<EditAction>,
}

impl TransactionGroup {
    /// Creates a new, empty transaction group.
    #[must_use]
    pub fn new(description: Option<String>) -> Self {
        Self {
            description,
            actions: Vec::new(),
        }
    }

    /// Adds an action to the transaction group.
    fn add_action(&mut self, action: EditAction) {
        self.actions.push(action);
    }

    /// Returns the actions in the group.
    #[must_use]
    pub fn actions(&self) -> &[EditAction] {
        &self.actions
    }

    /// Returns the description of the group, if any.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Represents a single entry in the edit history.
///
/// An entry can be either a single action or a group of actions (transaction).
#[derive(Debug, Clone)]
pub enum HistoryEntry {
    /// A single, atomic edit action.
    Single(EditAction),
    /// A group of actions treated as a single unit for undo/redo.
    Group(TransactionGroup),
}

/// Manages the undo/redo history for timeline edits.
#[derive(Debug, Clone, Default)]
pub struct EditHistory {
    /// Stack of actions that can be undone.
    undo_stack: Vec<HistoryEntry>,
    /// Stack of actions that can be redone.
    redo_stack: Vec<HistoryEntry>,
    /// Actions collected during an ongoing transaction.
    current_transaction: Option<TransactionGroup>,
    /// Maximum number of history entries to keep (optional).
    capacity: Option<usize>,
}

/// Error types specific to history operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum HistoryError {
    /// Attempted to undo when the undo stack is empty.
    #[error("Nothing to undo")]
    NothingToUndo,
    /// Attempted to redo when the redo stack is empty.
    #[error("Nothing to redo")]
    NothingToRedo,
    /// An error occurred while applying an action during undo/redo.
    #[error("Failed to apply action during undo/redo: {0}")]
    ApplyActionError(String),
    /// A transaction was already in progress.
    #[error("Transaction already in progress")]
    TransactionInProgress,
    /// No transaction is currently in progress.
    #[error("No transaction in progress")]
    NoTransactionInProgress,
}

pub type HistoryResult<T> = std::result::Result<T, HistoryError>;

impl EditHistory {
    /// Creates a new `EditHistory` with an optional capacity.
    #[must_use]
    pub fn new(capacity: Option<usize>) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    /// Starts a new transaction.
    ///
    /// All subsequent calls to `record_action` will add actions to this
    /// transaction until `commit_transaction` or `rollback_transaction` is called.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::TransactionInProgress` if a transaction is already active.
    pub fn begin_transaction(&mut self, description: Option<String>) -> HistoryResult<()> {
        if self.current_transaction.is_some() {
            return Err(HistoryError::TransactionInProgress);
        }
        self.current_transaction = Some(TransactionGroup::new(description));
        // Starting a transaction clears the redo stack
        self.redo_stack.clear();
        Ok(())
    }

    /// Commits the current transaction, adding it to the undo stack.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::NoTransactionInProgress` if no transaction is active.
    pub fn commit_transaction(&mut self) -> HistoryResult<()> {
        let transaction = self
            .current_transaction
            .take()
            .ok_or(HistoryError::NoTransactionInProgress)?;

        // Only add non-empty transactions to the history
        if !transaction.actions.is_empty() {
            self.push_entry(HistoryEntry::Group(transaction));
        }
        Ok(())
    }

    /// Rolls back (cancels) the current transaction.
    ///
    /// Any actions recorded since `begin_transaction` are discarded.
    /// This method does *not* automatically undo the actions applied during the transaction;
    /// the caller is responsible for reverting the state if necessary.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::NoTransactionInProgress` if no transaction is active.
    pub fn rollback_transaction(&mut self) -> HistoryResult<()> {
        if self.current_transaction.is_none() {
            return Err(HistoryError::NoTransactionInProgress);
        }
        self.current_transaction = None;
        // Note: Redo stack was already cleared when transaction started.
        Ok(())
    }

    /// Records a single edit action.
    ///
    /// If a transaction is in progress, the action is added to the transaction.
    /// Otherwise, the action is added directly to the undo stack as a single entry.
    /// Recording any action clears the redo stack.
    pub fn record_action(&mut self, action: EditAction) {
        if let Some(transaction) = self.current_transaction.as_mut() {
            transaction.add_action(action);
        } else {
            // Clear redo stack when a new action is recorded outside a transaction
            self.redo_stack.clear();
            self.push_entry(HistoryEntry::Single(action));
        }
    }

    /// Pushes a history entry onto the undo stack, managing capacity.
    fn push_entry(&mut self, entry: HistoryEntry) {
        self.undo_stack.push(entry);
        if let Some(cap) = self.capacity {
            if self.undo_stack.len() > cap {
                self.undo_stack.remove(0);
            }
        }
    }

    /// Undoes the last action or transaction, applying the changes to the timeline.
    ///
    /// Moves the undone entry to the redo stack.
    ///
    /// # Arguments
    ///
    /// * `timeline` - A mutable reference to the `Timeline` to apply the undo operation to.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::NothingToUndo` if the undo stack is empty.
    /// Returns `HistoryError::ApplyActionError` if applying the undo operation fails.
    pub fn undo(&mut self, timeline: &mut Timeline) -> HistoryResult<()> {
        let entry = self.undo_stack.pop().ok_or(HistoryError::NothingToUndo)?;

        // Apply the undo operation. This returns Result<(), TimelineError>
        let undo_result: Result<(), TimelineError> = match &entry {
            HistoryEntry::Single(action) => action.undo(timeline),
            HistoryEntry::Group(group) => {
                for action in group.actions().iter().rev() {
                    // Apply each action. If one fails, this will return Err(TimelineError)
                    action.undo(timeline)?;
                }
                // If loop completes, the group undo is successful at the Timeline level
                Ok(())
            }
        };

        match undo_result {
            Ok(_) => {
                self.redo_stack.push(entry);
                Ok(())
            }
            Err(timeline_error) => {
                // This is definitely TimelineError
                self.undo_stack.push(entry); // Push back the failed entry
                // Convert TimelineError to HistoryError before returning
                Err(HistoryError::from(timeline_error))
            }
        }
    }

    /// Redoes the last undone action or transaction, applying the changes to the timeline.
    ///
    /// Moves the redone entry back to the undo stack.
    ///
    /// # Arguments
    ///
    /// * `timeline` - A mutable reference to the `Timeline` to apply the redo operation to.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::NothingToRedo` if the redo stack is empty.
    /// Returns `HistoryError::ApplyActionError` if applying the redo operation fails.
    pub fn redo(&mut self, timeline: &mut Timeline) -> HistoryResult<()> {
        let entry = self.redo_stack.pop().ok_or(HistoryError::NothingToRedo)?;

        // Apply the redo operation. This returns Result<(), TimelineError>
        let redo_result: Result<(), TimelineError> = match &entry {
            HistoryEntry::Single(action) => action.apply(timeline),
            HistoryEntry::Group(group) => {
                for action in group.actions() {
                    // Apply each action. If one fails, this will return Err(TimelineError)
                    action.apply(timeline)?;
                }
                // If loop completes, the group redo is successful at the Timeline level
                Ok(())
            }
        };

        match redo_result {
            Ok(_) => {
                self.undo_stack.push(entry);
                Ok(())
            }
            Err(timeline_error) => {
                // This is definitely TimelineError
                self.redo_stack.push(entry); // Push back the failed entry
                // Convert TimelineError to HistoryError before returning
                Err(HistoryError::from(timeline_error))
            }
        }
    }

    /// Checks if there are any actions that can be undone.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Checks if there are any actions that can be redone.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears the entire edit history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_transaction = None;
    }

    /// Returns a reference to the undo stack.
    #[must_use]
    pub fn undo_stack(&self) -> &[HistoryEntry] {
        &self.undo_stack
    }

    /// Returns a reference to the redo stack.
    #[must_use]
    pub fn redo_stack(&self) -> &[HistoryEntry] {
        &self.redo_stack
    }
}

impl From<TimelineError> for HistoryError {
    fn from(err: TimelineError) -> Self {
        HistoryError::ApplyActionError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::timeline::{
        Clip,
        Timeline,
        Track,
        TrackId,
        TrackKind,
        // Fix: Remove the incorrect import alias for RelationshipKind
        // multi_track::{RelationshipKind, TrackRelationship}, // Also import here
        multi_track::TrackRelationship, // Ensure TrackRelationship is imported correctly
    };
    use crate::project::{AssetId, ClipId};
    use crate::utility::time::{Duration, TimePosition};
    // use std::collections::HashMap; // Keep if needed

    // Helper function to create a dummy clip
    fn create_dummy_clip(id: ClipId, asset_id: AssetId, pos: f64, dur: f64) -> Clip {
        Clip::new(
            id,
            asset_id,
            TimePosition::from_seconds(pos),
            Duration::from_seconds(dur),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(dur),
        )
    }

    // Helper function to create a dummy track
    fn create_dummy_track(id: TrackId, kind: TrackKind) -> Track {
        Track::new(id, kind)
    }

    #[test]
    fn test_new_history() {
        let history = EditHistory::new(Some(10));
        assert_eq!(history.capacity, Some(10));
        assert!(history.undo_stack.is_empty());
        assert!(history.redo_stack.is_empty());
        assert!(history.current_transaction.is_none());
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_record_single_action() {
        let mut history = EditHistory::new(None);
        let clip_id = ClipId::new();
        let track_id = TrackId::new();
        let action = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::from_seconds(0.0),
            new_position: TimePosition::from_seconds(10.0),
        };

        history.record_action(action.clone());

        assert_eq!(history.undo_stack.len(), 1);
        assert!(history.redo_stack.is_empty());
        assert!(history.can_undo());
        assert!(!history.can_redo());

        match &history.undo_stack[0] {
            HistoryEntry::Single(recorded_action) => {
                // Basic check, ideally use PartialEq if derived
                assert!(matches!(
                    recorded_action,
                    EditAction::SetClipPosition { .. }
                ));
            }
            _ => panic!("Expected Single entry"),
        }
    }

    #[test]
    fn test_undo_redo_single_action() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let clip = create_dummy_clip(clip_id, asset_id, 0.0, 5.0);
        let add_action = EditAction::AddClip { track_id, clip };

        // Apply and record
        add_action.apply(&mut timeline).unwrap();
        history.record_action(add_action);

        // Undo
        let undo_result = history.undo(&mut timeline);
        assert!(undo_result.is_ok());
        assert!(!history.can_undo());
        assert!(history.can_redo());
        assert_eq!(history.undo_stack.len(), 0);
        assert_eq!(history.redo_stack.len(), 1);
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_none()
        );
        assert!(matches!(
            history.redo_stack[0],
            HistoryEntry::Single(EditAction::AddClip { .. })
        ));

        // Redo
        let redo_result = history.redo(&mut timeline);
        assert!(redo_result.is_ok());
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_stack.len(), 1);
        assert_eq!(history.redo_stack.len(), 0);
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_some()
        );
        assert!(matches!(
            history.undo_stack[0],
            HistoryEntry::Single(EditAction::AddClip { .. })
        ));
    }

    #[test]
    fn test_record_clears_redo_stack() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let clip_id = ClipId::new();
        let track_kind = TrackKind::Video;
        let track_id = timeline.add_track(track_kind);
        let clip1_asset = AssetId::new();
        let clip1_start_pos = TimePosition::zero();
        let clip1_duration = Duration::from_seconds(10.0);
        let clip1 = create_dummy_clip(
            clip_id,
            clip1_asset,
            clip1_start_pos.as_seconds(),
            clip1_duration.as_seconds(),
        );
        timeline.add_clip(track_id, clip1).unwrap();

        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::zero(),
            new_position: TimePosition::from_seconds(5.0),
        };
        let action2 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::from_seconds(5.0),
            new_position: TimePosition::from_seconds(10.0),
        };

        // Apply action1 and record
        action1.apply(&mut timeline).unwrap();
        history.record_action(action1.clone());

        // Undo action1
        history.undo(&mut timeline).unwrap();
        assert!(history.can_redo());
        assert_eq!(history.redo_stack.len(), 1);
        assert_eq!(history.undo_stack.len(), 0);

        // Apply action2 and record
        action1.apply(&mut timeline).unwrap();
        action2.apply(&mut timeline).unwrap();
        history.record_action(action2.clone());

        assert!(!history.can_redo());
        assert_eq!(history.redo_stack.len(), 0);
        assert_eq!(history.undo_stack.len(), 1);
        assert!(matches!(
            history.undo_stack[0],
            HistoryEntry::Single(EditAction::SetClipPosition { .. })
        ));
    }

    #[test]
    fn test_transaction_commit() {
        let mut history = EditHistory::new(None);
        let clip_id = ClipId::new();
        let track_id = TrackId::new();
        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::zero(),
            new_position: TimePosition::from_seconds(5.0),
        };
        let action2 = EditAction::SetClipDuration {
            clip_id,
            track_id,
            original_duration: Duration::from_seconds(10.0),
            new_duration: Duration::from_seconds(8.0),
            original_source_end: TimePosition::from_seconds(10.0),
            new_source_end: TimePosition::from_seconds(8.0),
        };

        assert!(
            history
                .begin_transaction(Some("Clip Edit".to_string()))
                .is_ok()
        );
        history.record_action(action1.clone());
        history.record_action(action2.clone());
        assert!(history.commit_transaction().is_ok());

        assert_eq!(history.undo_stack.len(), 1);
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert!(history.current_transaction.is_none());

        match &history.undo_stack[0] {
            HistoryEntry::Group(group) => {
                assert_eq!(group.actions.len(), 2);
                assert_eq!(group.description(), Some("Clip Edit"));
                assert!(matches!(
                    group.actions[0],
                    EditAction::SetClipPosition { .. }
                ));
                assert!(matches!(
                    group.actions[1],
                    EditAction::SetClipDuration { .. }
                ));
            }
            _ => panic!("Expected Group entry"),
        }
    }

    #[test]
    fn test_transaction_rollback() {
        let mut history = EditHistory::new(None);
        let clip_id = ClipId::new();
        let track_id = TrackId::new();
        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::zero(),
            new_position: TimePosition::from_seconds(5.0),
        };

        assert!(history.begin_transaction(None).is_ok());
        history.record_action(action1.clone());
        assert!(history.rollback_transaction().is_ok());

        assert!(history.undo_stack.is_empty());
        assert!(history.redo_stack.is_empty());
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert!(history.current_transaction.is_none());
    }

    #[test]
    fn test_undo_redo_transaction() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        // Fix: Add track and clip first
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let initial_pos = TimePosition::zero();
        let initial_dur = Duration::from_seconds(10.0);
        let clip = create_dummy_clip(
            clip_id,
            asset_id,
            initial_pos.as_seconds(),
            initial_dur.as_seconds(),
        );
        timeline.add_clip(track_id, clip.clone()).unwrap(); // Add the clip to the timeline

        // Define actions based on the existing clip
        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: initial_pos, // Position before this action
            new_position: TimePosition::from_seconds(5.0),
        };
        let action2 = EditAction::SetClipDuration {
            clip_id,
            track_id,
            original_duration: initial_dur, // Duration before this action
            new_duration: Duration::from_seconds(8.0),
            // Need correct source end times based on clip/action definitions
            original_source_end: initial_pos + initial_dur,
            new_source_end: TimePosition::from_seconds(5.0) + Duration::from_seconds(8.0), // Assumes action1 was applied first
        };

        // --- Test Transaction --- (Apply actions *during* the transaction recording)
        history.begin_transaction(None).unwrap();

        // Apply and record action 1 (Set Position)
        action1.apply(&mut timeline).unwrap(); // Apply state change
        history.record_action(action1.clone()); // Record it
        assert_eq!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .unwrap()
                .position(),
            TimePosition::from_seconds(5.0)
        );

        // Apply and record action 2 (Set Duration)
        action2.apply(&mut timeline).unwrap(); // Apply state change
        history.record_action(action2.clone()); // Record it
        assert_eq!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .unwrap()
                .duration(),
            Duration::from_seconds(8.0)
        );

        history.commit_transaction().unwrap();

        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_stack.len(), 1);
        assert!(matches!(history.undo_stack[0], HistoryEntry::Group(_)));

        // --- Test Undo Transaction ---
        let undo_result = history.undo(&mut timeline);
        assert!(undo_result.is_ok());
        assert!(!history.can_undo());
        assert!(history.can_redo());

        // Check state reverted to *before* the transaction (clip at initial pos/dur)
        let reverted_clip = timeline
            .get_track(track_id)
            .unwrap()
            .get_clip(clip_id)
            .unwrap();
        assert_eq!(
            reverted_clip.position(),
            initial_pos,
            "Undo should revert position"
        );
        assert_eq!(
            reverted_clip.duration(),
            initial_dur,
            "Undo should revert duration"
        );
        assert!(matches!(history.redo_stack[0], HistoryEntry::Group(_)));

        // --- Test Redo Transaction ---
        let redo_result = history.redo(&mut timeline);
        assert!(redo_result.is_ok());
        assert!(history.can_undo());
        assert!(!history.can_redo());

        // Check state matches *after* the transaction (clip moved and resized)
        let redone_clip = timeline
            .get_track(track_id)
            .unwrap()
            .get_clip(clip_id)
            .unwrap();
        assert_eq!(
            redone_clip.position(),
            TimePosition::from_seconds(5.0),
            "Redo should restore position"
        );
        assert_eq!(
            redone_clip.duration(),
            Duration::from_seconds(8.0),
            "Redo should restore duration"
        );
        assert!(matches!(history.undo_stack[0], HistoryEntry::Group(_)));
    }

    #[test]
    fn test_history_apply_undo_single_action() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let clip = create_dummy_clip(clip_id, asset_id, 10.0, 5.0);
        let action = EditAction::AddClip {
            track_id,
            clip: clip.clone(),
        };

        // Apply action and record
        action.apply(&mut timeline).unwrap();
        history.record_action(action);
        assert!(history.can_undo());

        // Test Undo via history
        assert!(history.undo(&mut timeline).is_ok());
        assert!(!history.can_undo());
        assert!(history.can_redo());
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_none()
        );

        // Test Redo via history
        assert!(history.redo(&mut timeline).is_ok());
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_some()
        );
    }

    #[test]
    fn test_clear_history() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        // Fix: Add track and clip first
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let initial_pos = TimePosition::zero();
        let initial_dur = Duration::from_seconds(10.0);
        let clip = create_dummy_clip(
            clip_id,
            asset_id,
            initial_pos.as_seconds(),
            initial_dur.as_seconds(),
        );
        timeline.add_clip(track_id, clip.clone()).unwrap();

        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: initial_pos,
            new_position: TimePosition::from_seconds(5.0),
        };

        // Add an action
        action1.apply(&mut timeline).unwrap(); // Apply state change
        history.record_action(action1.clone());
        assert!(history.can_undo());

        // Undo it to populate redo stack
        history.undo(&mut timeline).unwrap();
        assert!(history.can_redo());

        // Start a transaction (can be empty for this test)
        assert!(history.begin_transaction(None).is_ok());
        // We don't need to record action1 again here for the clear test
        // history.record_action(action1);

        // Clear everything
        history.clear();

        assert!(history.undo_stack.is_empty());
        assert!(history.redo_stack.is_empty());
        assert!(history.current_transaction.is_none());
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    // Helper to create a simple timeline for testing apply/undo
    fn create_test_timeline() -> Timeline {
        Timeline::new()
    }

    // Example of an updated test (needs more work)
    #[test]
    fn test_apply_undo_add_clip() {
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let clip = create_dummy_clip(clip_id, asset_id, 10.0, 5.0);
        let action = EditAction::AddClip {
            track_id,
            clip: clip.clone(),
        };

        // Apply the action
        assert!(action.apply(&mut timeline).is_ok());
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_some()
        );

        // Undo the action
        assert!(action.undo(&mut timeline).is_ok());
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_none()
        );
    }

    #[test]
    fn test_set_track_name_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let original_name = timeline.get_track(track_id).unwrap().name().to_string();
        let new_name = "New Awesome Track Name".to_string();

        // Create the action
        let action = EditAction::SetTrackName {
            track_id,
            original_name: original_name.clone(),
            new_name: new_name.clone(),
        };

        // Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action);

        // Assert after apply
        assert_eq!(
            timeline.get_track(track_id).unwrap().name(),
            new_name,
            "Track name should be updated after apply"
        );

        // Undo
        history.undo(&mut timeline).unwrap();

        // Assert after undo
        assert_eq!(
            timeline.get_track(track_id).unwrap().name(),
            original_name,
            "Track name should revert after undo"
        );

        // Redo
        history.redo(&mut timeline).unwrap();

        // Assert after redo
        assert_eq!(
            timeline.get_track(track_id).unwrap().name(),
            new_name,
            "Track name should be updated again after redo"
        );
    }

    #[test]
    fn test_set_track_muted_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Audio);
        let original_muted = timeline.get_track(track_id).unwrap().is_muted(); // Should be false by default
        let new_muted = !original_muted; // Toggle the state

        // Create the action
        let action = EditAction::SetTrackMuted {
            track_id,
            original_muted,
            new_muted,
        };

        // Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action);

        // Assert after apply
        assert_eq!(
            timeline.get_track(track_id).unwrap().is_muted(),
            new_muted,
            "Track muted state should be updated after apply"
        );

        // Undo
        history.undo(&mut timeline).unwrap();

        // Assert after undo
        assert_eq!(
            timeline.get_track(track_id).unwrap().is_muted(),
            original_muted,
            "Track muted state should revert after undo"
        );

        // Redo
        history.redo(&mut timeline).unwrap();

        // Assert after redo
        assert_eq!(
            timeline.get_track(track_id).unwrap().is_muted(),
            new_muted,
            "Track muted state should be updated again after redo"
        );
    }

    #[test]
    fn test_set_track_locked_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let original_locked = timeline.get_track(track_id).unwrap().is_locked(); // Should be false by default
        let new_locked = !original_locked; // Toggle the state

        // Create the action
        let action = EditAction::SetTrackLocked {
            track_id,
            original_locked,
            new_locked,
        };

        // Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action);

        // Assert after apply
        assert_eq!(
            timeline.get_track(track_id).unwrap().is_locked(),
            new_locked,
            "Track locked state should be updated after apply"
        );

        // Undo
        history.undo(&mut timeline).unwrap();

        // Assert after undo
        assert_eq!(
            timeline.get_track(track_id).unwrap().is_locked(),
            original_locked,
            "Track locked state should revert after undo"
        );

        // Redo
        history.redo(&mut timeline).unwrap();

        // Assert after redo
        assert_eq!(
            timeline.get_track(track_id).unwrap().is_locked(),
            new_locked,
            "Track locked state should be updated again after redo"
        );
    }

    #[test]
    fn test_add_relationship_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track1_id = timeline.add_track(TrackKind::Video);
        let track2_id = timeline.add_track(TrackKind::Video);

        let relationship_kind = TrackRelationship::Locked;

        let action = EditAction::AddRelationship {
            source_id: track1_id,
            target_id: track2_id,
            relationship_kind,
        };

        // Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action.clone());

        // Assert after apply
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship should exist after apply"
        );
        assert_eq!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .unwrap(),
            TrackRelationship::Locked,
            "Relationship kind should be Lock after apply"
        );

        // Undo
        history.undo(&mut timeline).unwrap();

        // Assert after undo
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_none(),
            "Relationship should not exist after undo"
        );

        // Redo
        history.redo(&mut timeline).unwrap();

        // Assert after redo
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship should exist again after redo"
        );
        assert_eq!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .unwrap(),
            TrackRelationship::Locked,
            "Relationship kind should be Lock after redo"
        );
    }

    #[test]
    fn test_remove_relationship_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track1_id = timeline.add_track(TrackKind::Video);
        let track2_id = timeline.add_track(TrackKind::Video);

        let original_relationship_kind = TrackRelationship::Locked;

        timeline
            .multi_track_manager_mut()
            .add_relationship_no_timeline_check(track1_id, track2_id, original_relationship_kind)
            .unwrap();
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship setup failed"
        );

        let action = EditAction::RemoveRelationship {
            source_id: track1_id,
            target_id: track2_id,
            original_relationship_kind,
        };

        // Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action.clone());

        // Assert after apply
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_none(),
            "Relationship should be removed after apply"
        );

        // Undo
        history.undo(&mut timeline).unwrap();

        // Assert after undo
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship should be restored after undo"
        );
        assert_eq!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .unwrap(),
            TrackRelationship::Locked,
            "Restored relationship kind should be Lock"
        );

        // Redo
        history.redo(&mut timeline).unwrap();

        // Assert after redo
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_none(),
            "Relationship should be removed again after redo"
        );
    }

    #[test]
    fn test_update_relationship_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track1_id = timeline.add_track(TrackKind::Video);
        let track2_id = timeline.add_track(TrackKind::Audio);

        let original_relationship_kind = TrackRelationship::Locked;
        let updated_relationship_kind = TrackRelationship::TimingDependent;

        timeline
            .multi_track_manager_mut()
            .add_relationship_no_timeline_check(track1_id, track2_id, original_relationship_kind)
            .unwrap();

        let action = EditAction::UpdateRelationship {
            source_id: track1_id,
            target_id: track2_id,
            original_relationship_kind,
            updated_relationship_kind,
        };

        // Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action.clone());

        // Assert after apply (should have updated kind)
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship should exist after apply"
        );
        assert_eq!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .unwrap(),
            TrackRelationship::TimingDependent,
            "Relationship kind should be TimingDependent after apply"
        );

        // Undo
        history.undo(&mut timeline).unwrap();

        // Assert after undo (should have original kind)
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship should exist after undo"
        );
        assert_eq!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .unwrap(),
            TrackRelationship::Locked,
            "Relationship kind should revert to Lock after undo"
        );

        // Redo
        history.redo(&mut timeline).unwrap();

        // Assert after redo (should have updated kind again)
        assert!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .is_some(),
            "Relationship should exist after redo"
        );
        assert_eq!(
            timeline
                .multi_track_manager()
                .get_relationship(track1_id, track2_id)
                .unwrap(),
            TrackRelationship::TimingDependent,
            "Relationship kind should be TimingDependent again after redo"
        );
    }

    // Ensure multi_track_manager methods handle potential borrow issues when called from apply/undo (partially addressed, needs specific tests)

    // ... keep existing tests ...
}
