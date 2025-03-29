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
    /// Split a clip into two.
    SplitClip {
        track_id: TrackId,
        original_clip_id: ClipId,
        new_clip_id: ClipId, // ID of the second part created by the split
        split_position: TimePosition, // Position where the split occurred in the timeline
    },
    /// Merged two adjacent clips into one.
    MergeClips {
        track_id: TrackId,
        merged_clip_id: ClipId, // The ID of the clip that remains (was first_clip_id)
        removed_clip_id: ClipId, // The ID of the clip that was removed (was second_clip_id)
        // Store the original state of BOTH clips to properly undo the merge by splitting.
        original_merged_clip: Clip, // State of the first clip *before* merge
        removed_clip: Clip,         // The second clip that was removed
    },
    /// Changed the name of a track.
    SetTrackName {
        track_id: TrackId,
        original_name: String,
        new_name: String,
    },
    /// Changed the muted state of a track.
    SetTrackMuted {
        track_id: TrackId,
        original_muted: bool,
        new_muted: bool,
    },
    /// Changed the locked state of a track.
    SetTrackLocked {
        track_id: TrackId,
        original_locked: bool,
        new_locked: bool,
    },
    /// Added a relationship between two tracks.
    AddRelationship {
        source_id: TrackId,
        target_id: TrackId,
        relationship: super::multi_track::TrackRelationship,
    },
    /// Removed a relationship between two tracks.
    RemoveRelationship {
        source_id: TrackId,
        target_id: TrackId,
        original_relationship: super::multi_track::TrackRelationship, // Need the original to undo
    },
    /// Updated an existing relationship between two tracks.
    UpdateRelationship {
        source_id: TrackId,
        target_id: TrackId,
        original_relationship: super::multi_track::TrackRelationship,
        new_relationship: super::multi_track::TrackRelationship,
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
                let source_track_id =
                    timeline
                        .find_track_containing_clip(*clip_id)
                        .ok_or_else(|| {
                            // Use a more specific error if possible, or log the clip_id
                            TimelineError::InvalidOperation(format!(
                                "Cannot apply MoveClip: Source track for clip {} not found",
                                clip_id
                            ))
                        })?;

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
            EditAction::RemoveTrack { track_data, .. } => {
                // Remove the track by ID
                timeline.remove_track(track_data.id())
            }
            EditAction::SplitClip {
                track_id,
                original_clip_id,
                split_position,
                ..
            } => {
                // Apply split. The new_clip_id is generated by split_clip itself,
                // but we need it for undo, so it's stored in the action.
                // We assume the action stores the correct new_clip_id that *will* be generated.
                // A more robust system might involve returning the new ID from split_clip
                // and updating the action *after* the apply.
                timeline.split_clip(*track_id, *original_clip_id, *split_position)?; // Ignore the returned new_clip_id for apply
                Ok(())
            }
            EditAction::MergeClips {
                track_id,
                merged_clip_id,
                removed_clip_id,
                ..
            } => {
                // Apply merge.
                timeline.merge_clips(*track_id, *merged_clip_id, *removed_clip_id)
            }
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
            EditAction::AddRelationship {
                source_id,
                target_id,
                relationship,
            } => {
                // Check track existence *before* getting mutable borrow of manager
                if !timeline.has_track(*source_id) {
                    return Err(TimelineError::TrackNotFound(*source_id));
                }
                if !timeline.has_track(*target_id) {
                    return Err(TimelineError::TrackNotFound(*target_id));
                }
                // Use the new method that skips internal timeline checks
                timeline
                    .multi_track_manager_mut()
                    .add_relationship_no_timeline_check(*source_id, *target_id, *relationship)
                    .map_err(TimelineError::MultiTrack)
            }
            EditAction::RemoveRelationship {
                source_id,
                target_id,
                ..
            } => {
                // remove_relationship likely doesn't need &Timeline, so it should be fine.
                timeline
                    .multi_track_manager_mut()
                    .remove_relationship(*source_id, *target_id)
                    .map_err(TimelineError::MultiTrack)
            }
            EditAction::UpdateRelationship {
                source_id,
                target_id,
                original_relationship: _, // Original needed only for undo
                new_relationship,
            } => {
                // Check track existence *before* getting mutable borrow of manager
                if !timeline.has_track(*source_id) {
                    return Err(TimelineError::TrackNotFound(*source_id));
                }
                if !timeline.has_track(*target_id) {
                    return Err(TimelineError::TrackNotFound(*target_id));
                }
                // Apply as remove + add_no_timeline_check
                let manager = timeline.multi_track_manager_mut();
                manager.remove_relationship(*source_id, *target_id)?; // Propagate potential error
                // Use the new method that skips internal timeline checks
                manager
                    .add_relationship_no_timeline_check(*source_id, *target_id, *new_relationship)
                    .map_err(TimelineError::MultiTrack)
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
                // Undo removing a track by adding it back at the original index
                // Ensure index is valid
                let insert_index = (*original_index).min(timeline.tracks.len());
                timeline.tracks.insert(insert_index, track_data.clone());
                // TODO: Restore relationships?
                Ok(())
            }
            EditAction::SplitClip {
                track_id,
                original_clip_id,
                new_clip_id,
                ..
            } => {
                // Undo split by merging the two clips back together.
                // original_clip_id is the first part, new_clip_id is the second part.
                timeline.merge_clips(*track_id, *original_clip_id, *new_clip_id)
            }
            EditAction::MergeClips {
                track_id,
                merged_clip_id,
                original_merged_clip,
                removed_clip,
                ..
            } => {
                // Undo merge by:
                // 1. Restoring the state of the merged clip to its original pre-merge state.
                // 2. Re-inserting the removed clip.
                let track = timeline
                    .get_track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;

                // 1. Restore the first clip (the one that remained after merge)
                if let Some(clip_to_restore) = track.get_clip_mut(*merged_clip_id) {
                    // Restore its properties from original_merged_clip
                    *clip_to_restore = original_merged_clip.clone();
                } else {
                    // This shouldn't happen if state is consistent, but handle defensively.
                    return Err(TimelineError::ClipNotFound {
                        track: *track_id,
                        clip: *merged_clip_id,
                    });
                }

                // 2. Re-insert the removed clip
                // Use add_clip which handles sorting and overlap checks (though overlap shouldn't occur here)
                // We could also use track.clips.push and re-sort manually.
                track.add_clip(removed_clip.clone())?; // Use Track's add_clip
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
            EditAction::AddRelationship {
                source_id,
                target_id,
                ..
            } => {
                // remove_relationship should be fine
                timeline
                    .multi_track_manager_mut()
                    .remove_relationship(*source_id, *target_id)
                    .map_err(TimelineError::MultiTrack)
            }
            EditAction::RemoveRelationship {
                source_id,
                target_id,
                original_relationship,
            } => {
                // Check track existence *before* getting mutable borrow of manager
                if !timeline.has_track(*source_id) {
                    return Err(TimelineError::InvalidOperation(format!(
                        "Cannot undo RemoveRelationship: Source track {} not found",
                        source_id
                    )));
                }
                if !timeline.has_track(*target_id) {
                    return Err(TimelineError::InvalidOperation(format!(
                        "Cannot undo RemoveRelationship: Target track {} not found",
                        target_id
                    )));
                }
                // Undo removing by adding the original back, using the new method
                timeline
                    .multi_track_manager_mut()
                    .add_relationship_no_timeline_check(
                        *source_id,
                        *target_id,
                        *original_relationship,
                    )
                    .map_err(TimelineError::MultiTrack)
            }
            EditAction::UpdateRelationship {
                source_id,
                target_id,
                original_relationship,
                new_relationship: _, // New needed only for apply
            } => {
                // Check track existence *before* getting mutable borrow of manager
                if !timeline.has_track(*source_id) {
                    return Err(TimelineError::TrackNotFound(*source_id));
                }
                if !timeline.has_track(*target_id) {
                    return Err(TimelineError::TrackNotFound(*target_id));
                }
                // Undo updating by setting it back to the original (remove potentially new, add original)
                let manager = timeline.multi_track_manager_mut();
                // Assuming the relationship to remove *might* be the new one.
                let _ = manager.remove_relationship(*source_id, *target_id); // Ignore error if it didn't exist
                // Use the new method that skips internal timeline checks
                manager
                    .add_relationship_no_timeline_check(
                        *source_id,
                        *target_id,
                        *original_relationship,
                    )
                    .map_err(TimelineError::MultiTrack)
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

// Implement From<TimelineError> for HistoryError to allow use of `?`
impl From<TimelineError> for HistoryError {
    fn from(err: TimelineError) -> Self {
        HistoryError::ApplyActionError(err.to_string())
    }
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

        let result: Result<(), TimelineError> = match &entry {
            HistoryEntry::Single(action) => action.undo(timeline),
            HistoryEntry::Group(group) => {
                for action in group.actions().iter().rev() {
                    action.undo(timeline)?;
                }
                Ok(())
            }
        };

        match result {
            Ok(_) => {
                self.redo_stack.push(entry);
                Ok(())
            }
            Err(e) => {
                self.undo_stack.push(entry);
                Err(HistoryError::from(e))
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

        let result: Result<(), TimelineError> = match &entry {
            HistoryEntry::Single(action) => action.apply(timeline),
            HistoryEntry::Group(group) => {
                for action in group.actions() {
                    action.apply(timeline)?;
                }
                Ok(())
            }
        };

        match result {
            Ok(_) => {
                self.undo_stack.push(entry);
                Ok(())
            }
            Err(e) => {
                self.redo_stack.push(entry);
                Err(HistoryError::from(e))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::timeline::{Clip, Timeline, Track, TrackId, TrackKind};
    use crate::project::{AssetId, ClipId};
    use crate::utility::time::{Duration, TimePosition};
    use std::collections::HashMap;

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
        let mut timeline = create_test_timeline(); // Need timeline for apply/undo
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let clip = create_dummy_clip(clip_id, asset_id, 0.0, 5.0);
        let add_action = EditAction::AddClip { track_id, clip };

        // Apply and record
        add_action.apply(&mut timeline).unwrap();
        history.record_action(add_action);

        // Undo
        assert!(history.undo(&mut timeline).is_ok()); // Pass timeline
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

        // Redo
        assert!(history.redo(&mut timeline).is_ok()); // Pass timeline
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
    }

    #[test]
    fn test_record_clears_redo_stack() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let clip1 = create_dummy_clip(clip_id, AssetId::new(), 0.0, 5.0);
        // Assume action2 represents the state *after* action1 was applied
        let action1 = EditAction::AddClip {
            track_id,
            clip: clip1,
        };
        let action2 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::from_seconds(0.0),
            new_position: TimePosition::from_seconds(5.0),
        };

        // Apply action1 and record
        action1.apply(&mut timeline).unwrap();
        history.record_action(action1.clone());

        // Undo action1
        history.undo(&mut timeline).unwrap(); // Pass timeline
        assert!(history.can_redo());
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_none()
        ); // State after undo

        // Apply action2 and record
        // Need to ensure timeline state matches what action2 expects to *start* from.
        // Since action1 (AddClip) was undone, we need to re-apply it to simulate the state before action2.
        action1.apply(&mut timeline).unwrap(); // Re-apply AddClip state
        action2.apply(&mut timeline).unwrap(); // Apply the state change action2 represents (SetPosition)
        history.record_action(action2.clone()); // This should clear the redo stack

        assert!(!history.can_redo());
        assert_eq!(history.redo_stack.len(), 0);
        assert_eq!(history.undo_stack.len(), 1); // action2 is on undo stack
        assert_eq!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .unwrap()
                .position(),
            TimePosition::from_seconds(5.0)
        ); // Final state
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
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let asset_id = AssetId::new();
        let clip_start = create_dummy_clip(clip_id, asset_id, 0.0, 10.0);
        // Action 1: Add the clip
        let action1 = EditAction::AddClip {
            track_id,
            clip: clip_start,
        };
        // Action 2: Move the clip
        let action2 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::zero(),
            new_position: TimePosition::from_seconds(5.0),
        };

        // Apply initial state (clip added)
        action1.apply(&mut timeline).unwrap();

        // Record transaction: AddClip, then SetClipPosition
        history.begin_transaction(None).unwrap();
        history.record_action(action1.clone()); // Record adding
        history.record_action(action2.clone()); // Record moving
        history.commit_transaction().unwrap();

        // Apply the second action state (clip moved)
        action2.apply(&mut timeline).unwrap();
        assert_eq!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .unwrap()
                .position(),
            TimePosition::from_seconds(5.0)
        );

        // --- Test Undo ---
        assert!(history.undo(&mut timeline).is_ok()); // Pass timeline. Should undo both actions.
        assert!(!history.can_undo());
        assert!(history.can_redo());
        // Check state: Clip should be gone (undo AddClip was last in group undo)
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_none()
        );

        // --- Test Redo ---
        assert!(history.redo(&mut timeline).is_ok()); // Pass timeline. Should redo both actions.
        assert!(history.can_undo());
        assert!(!history.can_redo());
        // Check state: Clip should exist and be at the final position (redo SetClipPosition was last in group apply)
        assert_eq!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .unwrap()
                .position(),
            TimePosition::from_seconds(5.0)
        );

        // Verify the Group structure in the history stack after redo
        match &history.undo_stack[0] {
            HistoryEntry::Group(group) => assert_eq!(group.actions.len(), 2),
            _ => panic!("Redo failed or returned wrong type"),
        }
    }

    #[test]
    fn test_transaction_errors() {
        let mut history = EditHistory::new(None);
        assert!(history.begin_transaction(None).is_ok());
        assert_eq!(
            history.begin_transaction(None),
            Err(HistoryError::TransactionInProgress)
        );
        assert!(history.rollback_transaction().is_ok());
        assert_eq!(
            history.commit_transaction(),
            Err(HistoryError::NoTransactionInProgress)
        );
        assert_eq!(
            history.rollback_transaction(),
            Err(HistoryError::NoTransactionInProgress)
        );
    }

    #[test]
    fn test_history_capacity() {
        let mut history = EditHistory::new(Some(2));
        let clip_id = ClipId::new();
        let track_id = TrackId::new();

        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::zero(),
            new_position: TimePosition::from_seconds(1.0),
        };
        let action2 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::from_seconds(1.0),
            new_position: TimePosition::from_seconds(2.0),
        };
        let action3 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::from_seconds(2.0),
            new_position: TimePosition::from_seconds(3.0),
        };

        history.record_action(action1.clone());
        history.record_action(action2.clone());
        assert_eq!(history.undo_stack.len(), 2);

        history.record_action(action3.clone());
        assert_eq!(history.undo_stack.len(), 2); // Should have dropped action1

        // Verify that action1 is gone and action2, action3 remain
        match &history.undo_stack[0] {
            HistoryEntry::Single(EditAction::SetClipPosition { new_position, .. }) => {
                assert!((new_position.as_seconds() - 2.0).abs() < f64::EPSILON)
            }
            _ => panic!("Unexpected entry type or value at index 0"),
        }
        match &history.undo_stack[1] {
            HistoryEntry::Single(EditAction::SetClipPosition { new_position, .. }) => {
                assert!((new_position.as_seconds() - 3.0).abs() < f64::EPSILON)
            }
            _ => panic!("Unexpected entry type or value at index 1"),
        }
    }

    #[test]
    fn test_empty_transaction_commit() {
        let mut history = EditHistory::new(None);
        assert!(history.begin_transaction(None).is_ok());
        assert!(history.commit_transaction().is_ok());
        assert!(history.undo_stack.is_empty());
        assert!(!history.can_undo());
    }

    #[test]
    fn test_clear_history() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline(); // Need timeline
        let track_id = timeline.add_track(TrackKind::Video);
        let clip_id = ClipId::new();
        let clip = create_dummy_clip(clip_id, AssetId::new(), 0.0, 5.0);
        let action1 = EditAction::AddClip { track_id, clip };

        // Add an action
        action1.apply(&mut timeline).unwrap(); // Apply state change
        history.record_action(action1.clone());
        assert!(history.can_undo());

        // Undo it to populate redo stack
        history.undo(&mut timeline).unwrap(); // Pass timeline
        assert!(history.can_redo());

        // Start a transaction
        assert!(history.begin_transaction(None).is_ok());
        history.record_action(action1);

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

    // Example of an updated test using EditHistory
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
        assert!(
            timeline
                .get_track(track_id)
                .unwrap()
                .get_clip(clip_id)
                .is_some()
        );
        assert!(history.can_undo());

        // Test Undo via history
        assert!(history.undo(&mut timeline).is_ok()); // Pass timeline
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
        assert!(history.redo(&mut timeline).is_ok()); // Pass timeline
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
    fn test_split_clip_action_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let original_clip_id = ClipId::new();
        let new_clip_id = ClipId::new(); // Pre-generate ID for the second part
        let asset_id = AssetId::new();
        let original_pos = TimePosition::from_seconds(10.0);
        let original_dur = Duration::from_seconds(20.0);
        let split_time_rel = Duration::from_seconds(5.0); // Split 5 seconds into the clip
        let split_pos_abs = original_pos + split_time_rel;

        // 1. Setup: Add the initial clip
        let clip = create_dummy_clip(
            original_clip_id,
            asset_id,
            original_pos.as_seconds(),
            original_dur.as_seconds(),
        );
        timeline.add_clip(track_id, clip.clone()).unwrap();

        // 2. Create the SplitClip action
        // Note: The `apply` logic might internally generate a different new_clip_id.
        // The `undo` logic *needs* the correct `new_clip_id` as generated by the split.
        // For this test, we assume the pre-generated `new_clip_id` will be used or
        // that `merge_clips` during undo can find the correct second part based on `original_clip_id`.
        let action = EditAction::SplitClip {
            track_id,
            original_clip_id,
            new_clip_id, // Provide the ID expected for the second part
            split_position: split_pos_abs,
        };

        // 3. Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action);

        // 4. Assert state after apply
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(
            track.clips.len(),
            2,
            "Track should have two clips after split"
        );
        // Check first part (original ID, modified)
        let first_part = track.get_clip(original_clip_id).unwrap();
        assert_eq!(first_part.position(), original_pos, "First part position");
        assert_eq!(first_part.duration(), split_time_rel, "First part duration");
        // Check second part (we need to find it, maybe by position or assuming the ID)
        let second_part = track
            .clips
            .iter()
            .find(|c| c.id() != original_clip_id)
            .expect("Second part of split not found");
        // Let's use the found ID for consistency in further checks
        let actual_new_clip_id = second_part.id();
        assert_eq!(
            second_part.position(),
            split_pos_abs,
            "Second part position"
        );
        assert_eq!(
            second_part.duration(),
            original_dur - split_time_rel,
            "Second part duration"
        );

        // 5. Undo
        history.undo(&mut timeline).unwrap();

        // 6. Assert state after undo
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(
            track.clips.len(),
            1,
            "Track should have one clip after undo"
        );
        let restored_clip = track.get_clip(original_clip_id).unwrap();
        assert_eq!(
            restored_clip.position(),
            original_pos,
            "Restored clip position"
        );
        assert_eq!(
            restored_clip.duration(),
            original_dur,
            "Restored clip duration"
        );
        assert!(
            track.get_clip(actual_new_clip_id).is_none(),
            "Second part should be removed after undo"
        );

        // 7. Redo
        history.redo(&mut timeline).unwrap();

        // 8. Assert state after redo (should match state after apply)
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(
            track.clips.len(),
            2,
            "Track should have two clips after redo"
        );
        let first_part = track.get_clip(original_clip_id).unwrap();
        assert_eq!(
            first_part.duration(),
            split_time_rel,
            "First part duration after redo"
        );
        let second_part = track.get_clip(actual_new_clip_id).unwrap(); // Use the actual ID found earlier
        assert_eq!(
            second_part.position(),
            split_pos_abs,
            "Second part position after redo"
        );
        assert_eq!(
            second_part.duration(),
            original_dur - split_time_rel,
            "Second part duration after redo"
        );
    }

    #[test]
    fn test_merge_clips_action_apply_undo_redo() {
        let mut history = EditHistory::new(None);
        let mut timeline = create_test_timeline();
        let track_id = timeline.add_track(TrackKind::Video);
        let clip1_id = ClipId::new();
        let clip2_id = ClipId::new();
        let asset_id = AssetId::new();
        let clip1_pos = TimePosition::from_seconds(5.0);
        let clip1_dur = Duration::from_seconds(10.0);
        let clip2_pos = clip1_pos + clip1_dur; // Adjacent
        let clip2_dur = Duration::from_seconds(8.0);

        // 1. Setup: Add two adjacent clips
        let clip1 = create_dummy_clip(
            clip1_id,
            asset_id,
            clip1_pos.as_seconds(),
            clip1_dur.as_seconds(),
        );
        let clip2 = create_dummy_clip(
            clip2_id,
            asset_id,
            clip2_pos.as_seconds(),
            clip2_dur.as_seconds(),
        );
        timeline.add_clip(track_id, clip1.clone()).unwrap();
        timeline.add_clip(track_id, clip2.clone()).unwrap();
        assert_eq!(timeline.get_track(track_id).unwrap().clips.len(), 2);

        // 2. Create the MergeClips action
        let action = EditAction::MergeClips {
            track_id,
            merged_clip_id: clip1_id,    // Clip 1 remains
            removed_clip_id: clip2_id,   // Clip 2 is removed
            original_merged_clip: clip1, // State of clip 1 *before* merge
            removed_clip: clip2,         // The whole clip 2 that was removed
        };

        // 3. Apply and Record
        action.apply(&mut timeline).unwrap();
        history.record_action(action);

        // 4. Assert state after apply
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(
            track.clips.len(),
            1,
            "Track should have one clip after merge"
        );
        let merged_clip = track.get_clip(clip1_id).unwrap();
        assert_eq!(merged_clip.position(), clip1_pos, "Merged clip position");
        assert_eq!(
            merged_clip.duration(),
            clip1_dur + clip2_dur,
            "Merged clip duration"
        );
        assert!(
            track.get_clip(clip2_id).is_none(),
            "Second clip should be removed"
        );

        // 5. Undo
        history.undo(&mut timeline).unwrap();

        // 6. Assert state after undo
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(
            track.clips.len(),
            2,
            "Track should have two clips after undo"
        );
        // Check clip 1 restored
        let restored_clip1 = track.get_clip(clip1_id).unwrap();
        assert_eq!(
            restored_clip1.position(),
            clip1_pos,
            "Restored clip 1 position"
        );
        assert_eq!(
            restored_clip1.duration(),
            clip1_dur,
            "Restored clip 1 duration"
        );
        // Check clip 2 restored
        let restored_clip2 = track.get_clip(clip2_id).unwrap();
        assert_eq!(
            restored_clip2.position(),
            clip2_pos,
            "Restored clip 2 position"
        );
        assert_eq!(
            restored_clip2.duration(),
            clip2_dur,
            "Restored clip 2 duration"
        );

        // 7. Redo
        history.redo(&mut timeline).unwrap();

        // 8. Assert state after redo (should match state after apply)
        let track = timeline.get_track(track_id).unwrap();
        assert_eq!(
            track.clips.len(),
            1,
            "Track should have one clip after redo"
        );
        let merged_clip = track.get_clip(clip1_id).unwrap();
        assert_eq!(
            merged_clip.duration(),
            clip1_dur + clip2_dur,
            "Merged clip duration after redo"
        );
        assert!(
            track.get_clip(clip2_id).is_none(),
            "Second clip should be removed after redo"
        );
    }

    // TODO: Add tests for TrackRelationship actions (Add, Remove, Update) apply/undo
    // TODO: Add tests for Track property actions (SetName, SetMuted, SetLocked) apply/undo
    // TODO: Ensure multi_track_manager methods handle potential borrow issues when called from apply/undo (partially addressed, needs specific tests)

    // ... keep existing tests ...
}
