use crate::project::ClipId;
/// Timeline editing history and undo/redo functionality.
///
/// This module provides the structures and logic for managing
/// an undo/redo history for timeline operations, including support
/// for grouping actions into transactions.
use crate::project::timeline::{Clip, TimelineError, Track, TrackId};
use crate::utility::time::{Duration, TimePosition};

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
    // TODO: Add actions for Track property changes (mute, lock, name)
    // TODO: Add actions for TrackRelationship changes
    // TODO: Add actions for SplitClip, MergeClips
}

impl EditAction {
    // TODO: Implement apply and undo methods here or via a trait
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

    /// Undoes the last action or transaction.
    ///
    /// Moves the undone entry to the redo stack.
    /// The caller is responsible for applying the necessary state changes using
    /// the information within the returned `HistoryEntry`.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::NothingToUndo` if the undo stack is empty.
    pub fn undo(&mut self) -> HistoryResult<HistoryEntry> {
        let entry = self.undo_stack.pop().ok_or(HistoryError::NothingToUndo)?;
        self.redo_stack.push(entry.clone()); // Clone entry for redo stack
        // TODO: Implement actual undo logic application here, potentially returning results
        // For now, just return the entry - caller must apply undo
        Ok(entry)
    }

    /// Redoes the last undone action or transaction.
    ///
    /// Moves the redone entry back to the undo stack.
    /// The caller is responsible for applying the necessary state changes using
    /// the information within the returned `HistoryEntry`.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::NothingToRedo` if the redo stack is empty.
    pub fn redo(&mut self) -> HistoryResult<HistoryEntry> {
        let entry = self.redo_stack.pop().ok_or(HistoryError::NothingToRedo)?;
        self.undo_stack.push(entry.clone()); // Clone entry for undo stack
        // TODO: Implement actual redo logic application here, potentially returning results
        // For now, just return the entry - caller must apply redo
        Ok(entry)
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

// TODO: Implement apply/undo logic within Timeline based on returned HistoryEntry
// TODO: Add unit tests for EditHistory, including transactions

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::timeline::{Clip, Track, TrackId, TrackKind};
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
        let clip_id = ClipId::new();
        let track_id = TrackId::new();
        let action = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::from_seconds(0.0),
            new_position: TimePosition::from_seconds(10.0),
        };

        history.record_action(action.clone());

        // Undo
        let undo_result = history.undo();
        assert!(undo_result.is_ok());
        assert!(!history.can_undo());
        assert!(history.can_redo());
        assert_eq!(history.undo_stack.len(), 0);
        assert_eq!(history.redo_stack.len(), 1);

        if let Ok(HistoryEntry::Single(undone_action)) = undo_result {
            assert!(matches!(undone_action, EditAction::SetClipPosition { .. }));
        } else {
            panic!("Undo failed or returned wrong type");
        }

        // Redo
        let redo_result = history.redo();
        assert!(redo_result.is_ok());
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_stack.len(), 1);
        assert_eq!(history.redo_stack.len(), 0);

        if let Ok(HistoryEntry::Single(redone_action)) = redo_result {
            assert!(matches!(redone_action, EditAction::SetClipPosition { .. }));
        } else {
            panic!("Redo failed or returned wrong type");
        }
    }

    #[test]
    fn test_record_clears_redo_stack() {
        let mut history = EditHistory::new(None);
        let clip_id = ClipId::new();
        let track_id = TrackId::new();
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

        history.record_action(action1.clone());
        let _ = history.undo(); // Move action1 to redo stack
        assert!(history.can_redo());

        history.record_action(action2.clone()); // This should clear the redo stack
        assert!(!history.can_redo());
        assert_eq!(history.redo_stack.len(), 0);
        assert_eq!(history.undo_stack.len(), 1); // action2 is on undo stack
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

        // Record and commit transaction
        history.begin_transaction(None).unwrap();
        history.record_action(action1.clone());
        history.record_action(action2.clone());
        history.commit_transaction().unwrap();

        // Undo the transaction
        let undo_result = history.undo();
        assert!(undo_result.is_ok());
        assert!(!history.can_undo());
        assert!(history.can_redo());

        if let Ok(HistoryEntry::Group(group)) = undo_result {
            assert_eq!(group.actions.len(), 2);
        } else {
            panic!("Undo failed or returned wrong type");
        }

        // Redo the transaction
        let redo_result = history.redo();
        assert!(redo_result.is_ok());
        assert!(history.can_undo());
        assert!(!history.can_redo());

        if let Ok(HistoryEntry::Group(group)) = redo_result {
            assert_eq!(group.actions.len(), 2);
        } else {
            panic!("Redo failed or returned wrong type");
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
        let clip_id = ClipId::new();
        let track_id = TrackId::new();
        let action1 = EditAction::SetClipPosition {
            clip_id,
            track_id,
            original_position: TimePosition::zero(),
            new_position: TimePosition::from_seconds(5.0),
        };

        // Add an action
        history.record_action(action1.clone());
        assert!(history.can_undo());

        // Undo it to populate redo stack
        let _ = history.undo();
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
}
