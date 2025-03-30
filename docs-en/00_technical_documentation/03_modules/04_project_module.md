# edv - Project Module Implementation

This document provides detailed implementation guidelines for the Project module of the edv application.

## Overview

The Project module is the central component responsible for managing video editing project data. This includes:
- Defining the main `Project` structure.
- Handling project metadata (name, creation/modification dates, etc.).
- Managing project assets (video, audio, image files).
- Providing the core `Timeline` data structure and editing functionalities (tracks, clips, relationships, history).
- Implementing project serialization (saving/loading) in JSON format.
- Coordinating the rendering process for the project timeline.

## Structure

The Project module is organized as follows:

```
src/project/
├── mod.rs             # Module exports, Project struct, Asset*, ProjectMetadata, ProjectId, ProjectError
├── timeline/          # Timeline editing functionality
│   ├── mod.rs         # Timeline, Track, Clip structs, core editing methods (add/remove/split/merge/move)
│   ├── multi_track.rs # MultiTrackManager, TrackRelationship, dependency management
│   └── history.rs     # EditHistory, EditAction, TransactionGroup, undo/redo logic
├── rendering/         # Project rendering functionality
│   ├── mod.rs         # Rendering module exports, RenderError
│   ├── config.rs      # RenderConfig, VideoCodec, AudioCodec, OutputFormat
│   ├── compositor.rs  # TrackCompositor, track preparation and composition logic (using FFmpeg placeholders)
│   ├── pipeline.rs    # RenderPipeline, RenderResult, sync/async rendering execution
│   └── progress.rs    # RenderProgress, RenderStage, SharedProgressTracker
└── serialization/     # Project serialization functionality
    ├── mod.rs         # Serialization module exports
    └── json.rs        # JSON serializer/deserializer, Serialized* structs
```

**Note:** Asset management types (`AssetId`, `AssetReference`, `AssetMetadata`) are defined within `src/project/mod.rs`, not in a separate `asset` module.

## Key Components

### Project Structure (`mod.rs`)

The core `Project` struct holds all project-related data:

```rust
pub struct Project {
    /// Name of the project (also in project_metadata.name).
    pub name: String,
    /// Timeline of the project.
    pub timeline: timeline::Timeline,
    /// Assets used in the project.
    pub assets: Vec<AssetReference>,
    /// Additional metadata (currently unused, consider removing or integrating with ProjectMetadata).
    pub metadata: std::collections::HashMap<String, String>,
    /// Project metadata (name, timestamps, description, tags).
    pub project_metadata: ProjectMetadata,
}

impl Project {
    /// Creates a new empty project with the given name.
    pub fn new(name: &str) -> Self { /* ... */ }

    /// Adds an asset (represented by its path and metadata) to the project.
    /// Returns the newly generated AssetId.
    pub fn add_asset(&mut self, path: PathBuf, metadata: AssetMetadata) -> AssetId { /* ... */ }

    /// Gets an immutable reference to an asset by its ID.
    pub fn get_asset(&self, id: AssetId) -> Option<&AssetReference> { /* ... */ }

    /// Gets a mutable reference to an asset by its ID.
    pub fn get_asset_mut(&mut self, id: AssetId) -> Option<&mut AssetReference> { /* ... */ }

    /// Removes an asset from the project by its ID.
    /// Returns `Ok(())` or `ProjectError::AssetNotFound`.
    pub fn remove_asset(&mut self, id: AssetId) -> Result<()> { /* ... */ }

    /// Saves the project to a JSON file using the serialization module.
    /// Updates the modified timestamp before saving.
    pub fn save(&self, path: &std::path::Path) -> Result<()> { /* ... */ }

    /// Loads a project from a JSON file using the serialization module.
    pub fn load(path: &std::path::Path) -> Result<Self> { /* ... */ }

    /// Renders the project to a video file using default settings via the rendering module.
    pub fn render(&self, output_path: &std::path::Path) -> Result<rendering::RenderResult> { /* ... */ }

    /// Renders the project with the specified configuration via the rendering module.
    pub fn render_with_config(
        &self,
        config: rendering::RenderConfig,
    ) -> Result<rendering::RenderResult> { /* ... */ }
}

// Other related structs in mod.rs:
pub struct ProjectId(Uuid); // Unique ID for a project
pub struct ProjectMetadata { /* name, created_at, modified_at, description, tags */ }
pub struct AssetId(Uuid); // Unique ID for an asset
pub struct AssetReference { /* id, path, metadata */ }
pub struct AssetMetadata { /* duration, dimensions, asset_type, extra */ }

// Project-level error enum
pub enum ProjectError {
    Timeline(#[from] timeline::TimelineError),
    Io(#[from] std::io::Error),
    Serialization(String),
    AssetNotFound(AssetId),
    Rendering(#[from] rendering::RenderError),
}
```

**Responsibilities:**
- Holds the overall project state (timeline, assets, metadata).
- Provides methods for basic project lifecycle management (new, load, save).
- Manages the list of assets used in the project.
- Delegates timeline editing to the `Timeline` struct.
- Delegates rendering to the `rendering` module.
- Delegates serialization/deserialization to the `serialization` module.
- **Note:** Does not directly manage edit history (`EditHistory` is part of the `timeline` module).

### Timeline Editing (`timeline/mod.rs`, `timeline/multi_track.rs`, `timeline/history.rs`)

This is the core of the editing functionality.

#### Timeline Structure (`timeline/mod.rs`)

```rust
// Represents a single clip on a track
pub struct Clip {
    id: ClipId,
    asset_id: AssetId,
    position: TimePosition,  // Start time on the timeline
    duration: Duration,
    source_start: TimePosition, // Start time within the source asset
    source_end: TimePosition,   // End time within the source asset
}
impl Clip {
    // Methods: new, id, asset_id, position, end_position, duration,
    //          source_start, source_end, set_*, overlaps_with
}

// Represents a single track (Video, Audio, or Subtitle)
pub enum TrackKind { Video, Audio, Subtitle }
pub struct Track {
    id: TrackId,
    kind: TrackKind,
    name: String,
    clips: Vec<Clip>, // Sorted by position
    muted: bool,
    locked: bool,
}
impl Track {
    // Methods: new, id, kind, name, set_name, is_muted, set_muted,
    //          is_locked, set_locked, get_clips, get_clips_mut,
    //          get_clip, get_clip_mut, add_clip (checks overlap, sorts),
    //          remove_clip, duration (calculates based on last clip end)
}

// Represents the entire timeline with multiple tracks
pub struct Timeline {
    tracks: Vec<Track>,
    multi_track_manager: multi_track::MultiTrackManager,
}
impl Timeline {
    // Methods: new, get_tracks, get_track, get_track_mut, has_track,
    //          find_track_containing_clip, add_track, remove_track (updates manager),
    //          add_clip (delegates to Track), remove_clip (delegates to Track),
    //          multi_track_manager, multi_track_manager_mut, duration (calculates based on longest track),
    //          split_clip, merge_clips, move_clip_to_track
}

// Timeline-specific error enum
pub enum TimelineError {
    TrackNotFound(TrackId),
    ClipNotFound { track: TrackId, clip: ClipId },
    ClipOverlap { position: TimePosition },
    MultiTrack(#[from] multi_track::MultiTrackError),
    InvalidOperation(String),
}
```
**Responsibilities:**
- Defines the `Timeline`, `Track`, and `Clip` data structures.
- Provides methods for adding, removing, and querying tracks and clips.
- Implements core editing operations: `split_clip`, `merge_clips`, `move_clip_to_track`.
- Manages clip ordering within tracks.
- Handles potential errors like clip overlaps and invalid operations.
- Delegates multi-track relationship management to `MultiTrackManager`.

#### Multi-Track Relationships (`timeline/multi_track.rs`)

```rust
pub enum TrackRelationship { Independent, Locked, TimingDependent, VisibilityDependent }

pub struct MultiTrackManager {
    dependencies: HashMap<TrackId, HashMap<TrackId, TrackRelationship>>,
    reverse_dependencies: HashMap<TrackId, HashSet<TrackId>>,
}
impl MultiTrackManager {
    // Methods: new, add_relationship (checks cycles), remove_relationship,
    //          get_dependent_tracks, get_track_dependencies, get_relationship,
    //          apply_edit (propagates changes), remove_track,
    //          would_create_circular_dependency, is_dependent_on,
    //          propagate_changes (recursive propagation based on relationship type),
    //          synchronize_locked_tracks, update_timing_dependent_track,
    //          update_visibility_dependent_track
}

// Multi-track specific error enum
pub enum MultiTrackError { TrackNotFound(TrackId), CircularDependency(TrackId, TrackId), /* ... */ }
```
**Responsibilities:**
- Manages dependencies and relationships (`Locked`, `TimingDependent`, etc.) between tracks.
- Detects and prevents circular dependencies.
- Propagates changes made to one track to its dependent tracks based on the relationship type.
- Provides methods for querying track dependencies.

#### Edit History (`timeline/history.rs`)

```rust
pub enum EditAction { /* AddClip, RemoveClip, MoveClip, SetClipDuration, ..., AddRelationship, ... */ }
pub trait UndoableAction { fn apply(...); fn undo(...); }
impl UndoableAction for EditAction { /* ... */ }

pub struct TransactionGroup { description: Option<String>, actions: Vec<EditAction> }
pub enum HistoryEntry { Single(EditAction), Group(TransactionGroup) }

pub struct EditHistory {
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    current_transaction: Option<TransactionGroup>,
    capacity: Option<usize>,
}
impl EditHistory {
    // Methods: new, begin_transaction, commit_transaction, rollback_transaction,
    //          record_action, push_entry (handling capacity), undo, redo,
    //          can_undo, can_redo, clear, undo_stack, redo_stack
    //          peek_undo, peek_redo, shift_to_redo, shift_to_undo
}

// History-specific error enumeration
pub enum HistoryError { NothingToUndo, NothingToRedo, ApplyActionError(String), /* ... */ }
```
**Responsibilities:**
- Track timeline editing operations using `EditAction`.
- Implement undo and redo functionality for individual actions and transactions.
- Support grouping multiple actions into an atomic `TransactionGroup`.
- Manage undo and redo stacks.
- Handle potential errors during undo/redo operations.
- Provide methods to inspect the next undo/redo actions without applying them.
- Support direct manipulation of history stacks for efficient undo/redo implementation.

#### Enhanced Edit History Management

The Timeline module includes a sophisticated edit history management system with undo/redo operations and transaction grouping. Recent enhancements include:

##### Inspection and Advanced Stack Management

The `EditHistory` class now provides methods to peek at the next actions without executing them:

```rust
// Returns a reference to the next undo/redo action without removing it
pub fn peek_undo(&self) -> Option<&HistoryEntry>
pub fn peek_redo(&self) -> Option<&HistoryEntry>

// Direct movement of entries between undo and redo stacks
pub fn shift_to_redo(&mut self)  // Shifts the top entry from undo stack to redo stack
pub fn shift_to_undo(&mut self)  // Shifts the top entry from redo stack to undo stack
```

These methods enable more efficient and flexible undo/redo implementations, particularly useful when:
- Preview of undo/redo actions before application is needed
- Custom handling of specific action types is required
- Building UI features to display the next undo/redo action

##### Transaction Management

The timeline supports grouping multiple edit actions into a single transaction that can be undone or redone as a single unit:

```rust
// Start a new transaction group
timeline.begin_transaction(Some("Keyframe Animation".to_string()));

// Perform multiple operations as part of the transaction
timeline.add_keyframe_with_history(...); 
timeline.update_keyframe_with_history(...);
timeline.move_clip_to_track_with_history(...);

// Commit the transaction (all actions will be undone/redone together)
timeline.commit_transaction();

// Optionally cancel the transaction
timeline.rollback_transaction();
```

This transaction mechanism is particularly useful for:
- Complex operations involving multiple steps
- Ensuring related changes are treated as an atomic unit
- Improving user experience when undoing/redoing multi-step operations

##### Integration with Timeline Operations

The enhanced history system is integrated with all editing operations through `*_with_history` variants of regular editing methods:

```rust
// Standard operations
timeline.add_clip(track_id, clip);
timeline.remove_clip(track_id, clip_id);
timeline.move_clip_to_track(source_track_id, target_track_id, clip_id, new_position);

// History-tracking variants
timeline.add_clip_with_history(track_id, clip);
timeline.remove_clip_with_history(track_id, clip_id);
timeline.move_clip_to_track_with_history(source_track_id, target_track_id, clip_id, new_position);
```

This design provides flexibility, allowing operations to be performed with or without history tracking while ensuring consistent behavior throughout the application.

### Rendering (`rendering/`)

Handles the process of rendering the project timeline into a final video file.

```rust
// Configuration for rendering
pub struct RenderConfig { /* output_path, width, height, frame_rate, codecs, quality, format, range, threads, ... */ }
pub enum VideoCodec { H264, H265, VP9, ProRes, Copy }
pub enum AudioCodec { AAC, MP3, Opus, FLAC, Copy }
pub enum OutputFormat { MP4, WebM, MOV, MKV }

// Manages track composition
pub struct TrackCompositor { /* timeline, assets, intermediate_files, progress */ }
impl TrackCompositor {
    // Methods: new, set_progress_tracker, prepare_tracks (creates intermediate files via FFmpeg placeholders),
    //          composite_tracks (combines intermediate files via FFmpeg placeholders), compose (main entry point)
}

// Manages the overall rendering pipeline
pub struct RenderPipeline { /* project, config, progress, start_time */ }
impl RenderPipeline {
    // Methods: new, set_progress_callback, render (sync), render_async (async), cancel, get_progress
}

// Represents rendering progress
pub struct RenderProgress { /* total_frames, completed, position, duration, elapsed, estimated, fps, stage */ }
pub enum RenderStage { Preparing, RenderingVideo, ProcessingAudio, Muxing, Finalizing, Complete, Failed, Cancelled }
pub struct SharedProgressTracker { /* Arc<Mutex<ProgressTracker>> */ }

// Rendering specific error enum
pub enum RenderError { FFmpeg(String), Composition(String), Io(String), Timeline(String), Cancelled }
```
**Responsibilities:**
- Defines rendering configuration options (`RenderConfig`).
- Implements the rendering pipeline (`RenderPipeline`) coordinating different stages.
- Handles track composition (`TrackCompositor`), currently using placeholders for actual FFmpeg calls to create intermediate files for each track and then mux them together.
- Provides progress tracking (`RenderProgress`, `SharedProgressTracker`) and cancellation support.
- Defines rendering-specific errors (`RenderError`).

### Serialization (`serialization/`)

Handles saving and loading the project state to/from files.

```rust
// Serializable intermediate representations of project structures
struct SerializedProject { /* ... */ }
struct SerializedTimeline { /* ... */ }
// ... and other Serialized* structs

// Main serialization functions (currently only JSON)
pub fn serialize_project(project: &Project, path: &Path) -> Result<()> { /* ... */ }
pub fn deserialize_project(path: &Path) -> Result<Project> { /* ... */ }

// Serialization specific error enum
pub enum SerializationError { Io(#[from] std::io::Error), Json(#[from] serde_json::Error), IncompatibleFormat(String), UnsupportedVersion(String), /* ... */ }
```
**Responsibilities:**
- Serializes the `Project` state (including timeline, assets, metadata) into JSON format using intermediate `Serialized*` structs.
- Deserializes project data from JSON files back into a `Project` instance.
- Handles version checking and format validation during deserialization.
- Defines serialization-specific errors (`SerializationError`).
- **Note:** Binary serialization is mentioned in comments but not implemented.

## Dependencies

- `Project` uses `Timeline`, `AssetReference`, `ProjectMetadata`.
- `Timeline` uses `Track`, `Clip`, `MultiTrackManager`.
- `Track` uses `Clip`.
- `EditHistory` uses `EditAction`, `TransactionGroup`, `Timeline`.
- `RenderPipeline` uses `Project`, `RenderConfig`, `TrackCompositor`, `SharedProgressTracker`.
- `TrackCompositor` uses `Timeline`, `AssetReference`, `RenderConfig`, `SharedProgressTracker`, and (will use) `ffmpeg`.
- `Project` uses `serialize_project` / `deserialize_project`.
- `Serialization` uses `Project` and all its nested structures (via `Serialized*` representations) and `serde_json`.

## Implementation Details

- **IDs:** Uses `Uuid` wrapped in newtypes (`ProjectId`, `AssetId`, `ClipId`, `TrackId`) for unique identification.
- **Time:** Uses custom `TimePosition` and `Duration` types (likely from `utility` module, needs verification).
- **Error Handling:** Uses `thiserror` for defining custom error enums in each relevant submodule (`ProjectError`, `