# edv - Project Module Implementation

This document provides detailed implementation guidelines for the Project module of the edv application.

## Overview

The Project module manages the video editing project data structure, including timeline editing, clip management, project serialization/deserialization, and edit history tracking. It provides a comprehensive framework for non-linear video editing in a CLI environment.

## Structure

```
src/project/
├── mod.rs                 // Module exports
├── project.rs             // Core project management
├── timeline/              // Timeline data structures
│   ├── mod.rs             // Timeline exports
│   ├── timeline.rs        // Timeline implementation
│   ├── track.rs           // Track implementation
│   └── clip.rs            // Clip implementation
├── history/               // Edit history management
│   ├── mod.rs             // History exports
│   ├── action.rs          // Edit action implementation
│   └── history.rs         // History tracking
├── serialization/         // Project file format
│   ├── mod.rs             // Serialization exports
│   ├── json.rs            // JSON serializer/deserializer
│   └── binary.rs          // Binary serializer/deserializer
├── operations/            // Project operations
│   ├── mod.rs             // Operations exports
│   ├── clip_ops.rs        // Clip operations
│   ├── track_ops.rs       // Track operations
│   └── transition_ops.rs  // Transition operations
└── utils/                 // Project-specific utilities
    ├── mod.rs             // Utility exports
    ├── validation.rs      // Project validation
    └── optimizer.rs       // Timeline optimization
```

## Key Components

### Project Manager (project.rs)

The central component for managing video editing projects:

```rust
pub struct Project {
    /// Unique project identifier
    pub id: ProjectId,
    /// Project metadata
    pub metadata: ProjectMetadata,
    /// Timeline data
    pub timeline: Timeline,
    /// Assets used in the project
    pub assets: Vec<AssetReference>,
    /// Edit history
    pub history: EditHistory,
    /// Project settings
    pub settings: ProjectSettings,
}

impl Project {
    /// Create a new empty project
    pub fn new(name: &str) -> Self {
        let id = ProjectId::new();
        let now = Utc::now();
        
        Self {
            id,
            metadata: ProjectMetadata {
                name: name.to_string(),
                created_at: now,
                modified_at: now,
                description: String::new(),
                tags: Vec::new(),
            },
            timeline: Timeline::new(),
            assets: Vec::new(),
            history: EditHistory::new(),
            settings: ProjectSettings::default(),
        }
    }
    
    /// Save project to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Serialize project and save to file
        let serializer = match self.settings.serialization_format {
            SerializationFormat::Json => JsonSerializer::new(),
            SerializationFormat::Binary => BinarySerializer::new(),
        };
        
        serializer.serialize(self, path)
    }
    
    /// Load project from file
    pub fn load(path: &Path) -> Result<Self> {
        // Determine format from file extension
        let format = SerializationFormat::from_path(path)?;
        
        // Deserialize project from file
        let deserializer = match format {
            SerializationFormat::Json => JsonDeserializer::new(),
            SerializationFormat::Binary => BinaryDeserializer::new(),
        };
        
        deserializer.deserialize(path)
    }
    
    /// Add asset to project
    pub fn add_asset(&mut self, asset: &Asset) -> Result<AssetId> {
        let asset_ref = AssetReference {
            id: asset.id,
            path: asset.path.clone(),
            metadata: asset.metadata.clone(),
        };
        
        if self.assets.iter().any(|a| a.id == asset.id) {
            return Err(Error::DuplicateAsset(asset.id));
        }
        
        self.assets.push(asset_ref);
        self.record_history_action(HistoryAction::AddAsset(asset.id));
        Ok(asset.id)
    }
    
    /// Remove asset from project
    pub fn remove_asset(&mut self, asset_id: AssetId) -> Result<()> {
        // Check if asset is used in timeline
        if self.timeline.is_asset_used(asset_id) {
            return Err(Error::AssetInUse(asset_id));
        }
        
        let idx = self.assets.iter()
            .position(|a| a.id == asset_id)
            .ok_or(Error::AssetNotFound(asset_id))?;
            
        self.assets.remove(idx);
        self.record_history_action(HistoryAction::RemoveAsset(asset_id));
        Ok(())
    }
    
    /// Access the timeline
    pub fn timeline(&mut self) -> &mut Timeline {
        &mut self.timeline
    }
    
    /// Undo last operation
    pub fn undo(&mut self) -> Result<()> {
        self.history.undo(self)
    }
    
    /// Redo previously undone operation
    pub fn redo(&mut self) -> Result<()> {
        self.history.redo(self)
    }
    
    /// Record action in history
    fn record_history_action(&mut self, action: HistoryAction) {
        self.history.record(action);
        self.metadata.modified_at = Utc::now();
    }
}
```

### Timeline (timeline/timeline.rs)

The timeline data structure for managing video editing timeline:

```rust
pub struct Timeline {
    /// Tracks in the timeline
    pub tracks: Vec<Track>,
    /// Timeline duration
    pub duration: Duration,
    /// Timeline settings
    pub settings: TimelineSettings,
}

impl Timeline {
    /// Create a new empty timeline
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            duration: Duration::from_seconds(0.0),
            settings: TimelineSettings::default(),
        }
    }
    
    /// Add a new track to the timeline
    pub fn add_track(&mut self, kind: TrackKind) -> TrackId {
        let id = TrackId::new();
        let track = Track::new(id, kind);
        self.tracks.push(track);
        id
    }
    
    /// Remove a track from the timeline
    pub fn remove_track(&mut self, track_id: TrackId) -> Result<()> {
        let idx = self.tracks.iter()
            .position(|t| t.id == track_id)
            .ok_or(Error::TrackNotFound(track_id))?;
            
        self.tracks.remove(idx);
        self.recalculate_duration();
        Ok(())
    }
    
    /// Add a clip to a track
    pub fn add_clip(&mut self, track_id: TrackId, clip: Clip) -> Result<ClipId> {
        let track = self.tracks.iter_mut()
            .find(|t| t.id == track_id)
            .ok_or(Error::TrackNotFound(track_id))?;
            
        // Check for overlapping clips
        if track.has_overlap(&clip) {
            return Err(Error::ClipOverlap(clip.start_time));
        }
        
        let clip_id = clip.id;
        track.add_clip(clip);
        self.recalculate_duration();
        Ok(clip_id)
    }
    
    /// Remove a clip from a track
    pub fn remove_clip(&mut self, track_id: TrackId, clip_id: ClipId) -> Result<()> {
        let track = self.tracks.iter_mut()
            .find(|t| t.id == track_id)
            .ok_or(Error::TrackNotFound(track_id))?;
            
        track.remove_clip(clip_id)?;
        self.recalculate_duration();
        Ok(())
    }
    
    /// Move a clip within the timeline
    pub fn move_clip(&mut self, track_id: TrackId, clip_id: ClipId, new_start: TimePosition) -> Result<()> {
        let track = self.tracks.iter_mut()
            .find(|t| t.id == track_id)
            .ok_or(Error::TrackNotFound(track_id))?;
            
        track.move_clip(clip_id, new_start)?;
        self.recalculate_duration();
        Ok(())
    }
    
    /// Check if an asset is used in the timeline
    pub fn is_asset_used(&self, asset_id: AssetId) -> bool {
        self.tracks.iter().any(|track| {
            track.clips.iter().any(|clip| clip.asset_id == asset_id)
        })
    }
    
    /// Recalculate timeline duration based on clips
    fn recalculate_duration(&mut self) {
        self.duration = self.tracks.iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.start_time + c.duration)
            .max()
            .unwrap_or(TimePosition::from_seconds(0.0))
            .into();
    }
}
```

### Track (timeline/track.rs)

The track structure for managing a single track in the timeline:

```rust
pub struct Track {
    /// Track identifier
    pub id: TrackId,
    /// Track type (video, audio, etc.)
    pub kind: TrackKind,
    /// Clips in this track
    pub clips: Vec<Clip>,
    /// Whether the track is enabled
    pub enabled: bool,
    /// Track name
    pub name: String,
}

impl Track {
    /// Create a new empty track
    pub fn new(id: TrackId, kind: TrackKind) -> Self {
        Self {
            id,
            kind,
            clips: Vec::new(),
            enabled: true,
            name: format!("{} {}", kind.as_str(), id),
        }
    }
    
    /// Add a clip to the track
    pub fn add_clip(&mut self, clip: Clip) {
        // Insert clip in sorted order by start time
        let idx = self.clips.binary_search_by(|c| {
            c.start_time.partial_cmp(&clip.start_time).unwrap()
        }).unwrap_or_else(|e| e);
        
        self.clips.insert(idx, clip);
    }
    
    /// Remove a clip from the track
    pub fn remove_clip(&mut self, clip_id: ClipId) -> Result<()> {
        let idx = self.clips.iter()
            .position(|c| c.id == clip_id)
            .ok_or(Error::ClipNotFound(clip_id))?;
            
        self.clips.remove(idx);
        Ok(())
    }
    
    /// Move a clip to a new start position
    pub fn move_clip(&mut self, clip_id: ClipId, new_start: TimePosition) -> Result<()> {
        // Find clip
        let idx = self.clips.iter()
            .position(|c| c.id == clip_id)
            .ok_or(Error::ClipNotFound(clip_id))?;
            
        // Remove clip from current position
        let mut clip = self.clips.remove(idx);
        
        // Update start time
        clip.start_time = new_start;
        
        // Check for overlaps with other clips
        if self.has_overlap(&clip) {
            // Restore clip to original position
            self.clips.insert(idx, clip);
            return Err(Error::ClipOverlap(new_start));
        }
        
        // Add clip at new position
        self.add_clip(clip);
        Ok(())
    }
    
    /// Check if a clip would overlap with existing clips
    pub fn has_overlap(&self, clip: &Clip) -> bool {
        self.clips.iter().any(|c| {
            c.id != clip.id && 
            c.start_time < clip.start_time + clip.duration &&
            c.start_time + c.duration > clip.start_time
        })
    }
}
```

### Clip (timeline/clip.rs)

The clip structure representing a media segment in the timeline:

```rust
pub struct Clip {
    /// Clip identifier
    pub id: ClipId,
    /// Associated asset identifier
    pub asset_id: AssetId,
    /// Start time in timeline
    pub start_time: TimePosition,
    /// Clip duration
    pub duration: Duration,
    /// Start point in source asset
    pub in_point: TimePosition,
    /// End point in source asset
    pub out_point: TimePosition,
    /// Applied effects
    pub effects: Vec<Effect>,
    /// Clip properties
    pub properties: ClipProperties,
}

impl Clip {
    /// Create a new clip from an asset
    pub fn new(
        asset_id: AssetId, 
        start_time: TimePosition,
        in_point: TimePosition,
        out_point: TimePosition,
    ) -> Result<Self> {
        if out_point <= in_point {
            return Err(Error::InvalidTimeRange(in_point, out_point));
        }
        
        let duration = Duration::from_time_diff(in_point, out_point);
        
        Ok(Self {
            id: ClipId::new(),
            asset_id,
            start_time,
            duration,
            in_point,
            out_point,
            effects: Vec::new(),
            properties: ClipProperties::default(),
        })
    }
    
    /// Add an effect to the clip
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }
    
    /// Remove an effect from the clip
    pub fn remove_effect(&mut self, effect_id: EffectId) -> Result<()> {
        let idx = self.effects.iter()
            .position(|e| e.id == effect_id)
            .ok_or(Error::EffectNotFound(effect_id))?;
            
        self.effects.remove(idx);
        Ok(())
    }
    
    /// Change clip duration and adjust in/out points
    pub fn change_duration(&mut self, new_duration: Duration) -> Result<()> {
        // Ensure new duration is valid
        if new_duration <= Duration::from_seconds(0.0) {
            return Err(Error::InvalidDuration(new_duration));
        }
        
        let original_asset_duration = self.out_point - self.in_point;
        let scale_factor = new_duration.as_seconds() / original_asset_duration.as_seconds();
        
        // Adjust in/out points to maintain relative positions
        self.duration = new_duration;
        self.out_point = self.in_point + new_duration;
        
        Ok(())
    }
}
```

### Edit History (history/history.rs)

The history tracking system for undo/redo functionality:

```rust
pub struct EditHistory {
    /// History of actions
    actions: Vec<HistoryAction>,
    /// Current position in history
    current_index: usize,
    /// Maximum history size
    max_history: usize,
}

impl EditHistory {
    /// Create a new empty history
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            current_index: 0,
            max_history: 100, // Default size
        }
    }
    
    /// Record a new action
    pub fn record(&mut self, action: HistoryAction) {
        // If we're not at the end of history, truncate the future actions
        if self.current_index < self.actions.len() {
            self.actions.truncate(self.current_index);
        }
        
        // Add new action
        self.actions.push(action);
        self.current_index += 1;
        
        // Trim history if needed
        if self.actions.len() > self.max_history {
            self.actions.remove(0);
            self.current_index -= 1;
        }
    }
    
    /// Undo the last action
    pub fn undo(&mut self, project: &mut Project) -> Result<()> {
        if self.current_index == 0 {
            return Err(Error::NoActionToUndo);
        }
        
        self.current_index -= 1;
        let action = &self.actions[self.current_index];
        action.undo(project)
    }
    
    /// Redo a previously undone action
    pub fn redo(&mut self, project: &mut Project) -> Result<()> {
        if self.current_index >= self.actions.len() {
            return Err(Error::NoActionToRedo);
        }
        
        let action = &self.actions[self.current_index];
        action.redo(project)?;
        self.current_index += 1;
        Ok(())
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.actions.clear();
        self.current_index = 0;
    }
}
```

### History Action (history/action.rs)

The action data structure for tracking edits:

```rust
pub enum HistoryAction {
    /// Add clip to timeline
    AddClip {
        track_id: TrackId,
        clip: Clip,
    },
    /// Remove clip from timeline
    RemoveClip {
        track_id: TrackId,
        clip: Clip,
    },
    /// Move clip in timeline
    MoveClip {
        track_id: TrackId,
        clip_id: ClipId,
        old_start: TimePosition,
        new_start: TimePosition,
    },
    /// Add track to timeline
    AddTrack {
        track: Track,
    },
    /// Remove track from timeline
    RemoveTrack {
        track: Track,
    },
    /// Add asset to project
    AddAsset(AssetId),
    /// Remove asset from project
    RemoveAsset(AssetId),
    /// Compound action (multiple actions as one)
    Compound(Vec<HistoryAction>),
}

impl HistoryAction {
    /// Undo this action
    pub fn undo(&self, project: &mut Project) -> Result<()> {
        match self {
            Self::AddClip { track_id, clip } => {
                project.timeline().remove_clip(*track_id, clip.id)
            },
            Self::RemoveClip { track_id, clip } => {
                project.timeline().add_clip(*track_id, clip.clone()).map(|_| ())
            },
            Self::MoveClip { track_id, clip_id, old_start, .. } => {
                project.timeline().move_clip(*track_id, *clip_id, *old_start)
            },
            Self::AddTrack { track } => {
                project.timeline().remove_track(track.id)
            },
            Self::RemoveTrack { track } => {
                // Re-add track with all its clips
                let timeline = project.timeline();
                timeline.tracks.push(track.clone());
                Ok(())
            },
            Self::AddAsset(asset_id) => {
                project.remove_asset(*asset_id)
            },
            Self::RemoveAsset(asset_id) => {
                // This requires the asset to be cached somewhere
                // In a real implementation, we'd need to store the full asset data
                Err(Error::UndoNotSupported("RemoveAsset"))
            },
            Self::Compound(actions) => {
                // Undo actions in reverse order
                for action in actions.iter().rev() {
                    action.undo(project)?;
                }
                Ok(())
            },
        }
    }
    
    /// Redo this action
    pub fn redo(&self, project: &mut Project) -> Result<()> {
        match self {
            Self::AddClip { track_id, clip } => {
                project.timeline().add_clip(*track_id, clip.clone()).map(|_| ())
            },
            Self::RemoveClip { track_id, clip } => {
                project.timeline().remove_clip(*track_id, clip.id)
            },
            Self::MoveClip { track_id, clip_id, new_start, .. } => {
                project.timeline().move_clip(*track_id, *clip_id, *new_start)
            },
            Self::AddTrack { track } => {
                // Just add track ID since we can't really "redo" adding the exact same track
                project.timeline().add_track(track.kind);
                Ok(())
            },
            Self::RemoveTrack { track } => {
                project.timeline().remove_track(track.id)
            },
            Self::AddAsset(asset_id) => {
                // Similar problem as RemoveAsset undo
                Err(Error::RedoNotSupported("AddAsset"))
            },
            Self::RemoveAsset(asset_id) => {
                project.remove_asset(*asset_id)
            },
            Self::Compound(actions) => {
                // Redo actions in original order
                for action in actions {
                    action.redo(project)?;
                }
                Ok(())
            },
        }
    }
}
```

## Key Interfaces

### Project Interface

The Project module exposes the following key interfaces:

- **Project Management Interface**: Create, load, save, and manage projects
- **Timeline Editing Interface**: Add, remove, and edit clips and tracks
- **Asset Reference Interface**: Reference and use media assets within projects
- **History Management Interface**: Undo and redo operations

### Serialization Interface

The serialization system provides interfaces for:

- **Project Serialization**: Convert projects to persistent formats
- **Project Deserialization**: Load projects from stored formats
- **Format Conversion**: Convert between different serialization formats

### Track Relationship Serialization

The track relationship serialization system provides a robust mechanism for persisting complex timeline structures:

#### Serialized Representation

- **SerializedTrackRelationship**: Enumeration representing different types of track relationships
  - Independent: Tracks with no synchronization requirements
  - Locked: Tracks that should move and edit together
  - TimingDependent: Tracks where one affects the timing of another
  - VisibilityDependent: Tracks where one determines visibility of another

- **SerializedMultiTrackManager**: Container for track relationship mappings
  - Maps string-based track IDs to their relationship targets
  - Preserves relationship types between tracks

#### Serialization Process

The serialization process for track relationships follows these steps:

1. **Relationship Collection**: Retrieves all track relationships from the MultiTrackManager
2. **ID Conversion**: Converts TrackId objects to string representation
3. **Relationship Mapping**: Creates a mapping of source tracks to target tracks with relationship types
4. **JSON Encoding**: Encodes the relationship structure into the JSON format

#### Deserialization Process

The deserialization process follows these steps:

1. **Track Restoration**: Rebuilds all tracks in the timeline
2. **ID Mapping**: Creates a mapping between string IDs and actual TrackId objects
3. **Relationship Validation**: Verifies relationships for validity (avoids circular dependencies)
4. **Safe Restoration**: Adds valid relationships to the timeline using a pattern that avoids borrowing conflicts
5. **Error Handling**: Uses a best-effort approach to restore as many relationships as possible

This robust serialization system ensures that complex timeline structures with multiple interdependent tracks can be saved and restored accurately, providing a foundation for sophisticated video editing capabilities.

### Timeline Operations Interface

The timeline operations system provides interfaces for:

- **Clip Operations**: Add, remove, trim, split clips
- **Track Operations**: Add, remove, reorder tracks
- **Timeline Navigation**: Position cursor, zoom, scroll

## Performance Considerations

- **Memory Efficiency**: Project structures are designed to minimize memory usage
- **Fast Serialization**: Efficient serialization formats for large projects
- **Undo/Redo Performance**: History operations optimized for frequent use
- **Timeline Rendering**: Efficient time-based queries for timeline rendering

## Future Enhancements

- **Multi-Track Compositing**: Advanced compositing between video tracks
- **Keyframe Support**: Keyframe animation for effects and properties
- **Nested Timelines**: Support for sequences within sequences
- **Enhanced Effects**: Plugin system for custom effects
- **Version Control**: Git-like version control for projects
- **Collaborative Editing**: Support for multiple editors working on the same project

This modular Project implementation provides a solid foundation for sophisticated video editing capabilities while maintaining a clean and efficient architecture suitable for a CLI-based tool.

## Testing Strategy

This comprehensive testing strategy ensures that the Project module is thoroughly tested for correctness, reliability, and performance, providing a solid foundation for the non-linear editing capabilities of the edv application. 

## Implementation Status Update (2024)

### Current Implementation Status: IN PROGRESS (50%)

The Project module is currently under active development as part of Phase 2 of the edv project. While the core architecture and data structures have been designed, the implementation is still in progress with varying levels of completion across components:

| Component | Status | Implementation Level | Notes |
|-----------|--------|----------------------|-------|
| Project Structure | ✅ Complete | 90% | Core project data structure implemented |
| Timeline Model - Basic | ✅ Complete | 85% | Single track timeline functioning |
| Timeline Model - Multi-track | 🔄 In Progress | 60% | Track relationships implemented, advanced features in progress |
| Clip Management | ✅ Complete | 80% | Basic clip operations implemented |
| Project Serialization | 🔄 In Progress | 70% | JSON format implemented, track relationship serialization complete |
| Edit History | 🔄 In Progress | 40% | Basic action recording, undo/redo partially implemented |
| Timeline Operations | 🔄 In Progress | 40% | Basic operations implemented, advanced features pending |
| Project Validation | 🔄 In Progress | 30% | Basic validation implemented |

### Implementation Priorities

The current implementation focus is on:

1. **Advanced Timeline Functionality**
   - Enhancing multi-track timeline capabilities
   - Implementing complex track relationships
   - Developing comprehensive validation for timeline operations

2. **Project Persistence**
   - Optimizing the JSON serialization format
   - Implementing incremental save operations
   - Enhancing load performance for large projects

3. **Basic Edit History**
   - Implementing fundamental undo/redo operations
   - Ensuring state consistency during history navigation
   - Developing a stable history action model

### Key Achievements

Several major components have been successfully implemented:

1. **Multi-track Relationship Management**
   - Complete implementation of track relationship data structures
   - Support for different relationship types (Locked, TimingDependent, VisibilityDependent)
   - Circular dependency detection and prevention

2. **Serialization System**
   - Complete serialization of timeline structures including track relationships
   - Robust deserialization with error handling
   - Best-effort restoration of valid relationships

3. **Timeline Data Model**
   - Flexible track and clip model that supports complex editing operations
   - Efficient query mechanisms for timeline data
   - Support for various media types in a unified timeline

### Key Challenges

Several challenges have been encountered during implementation:

1. **Timeline Model Complexity**
   - Balancing flexibility with simplicity in the timeline data model
   - Ensuring efficient operations for large timelines
   - Maintaining consistent state during complex operations

2. **Borrowing and Ownership**
   - Addressing mutable borrowing conflicts in track relationship management
   - Implementing safe patterns for timeline manipulation
   - Resolving reference issues during serialization/deserialization

3. **Edit History Design**
   - Designing a comprehensive yet efficient history system
   - Determining the appropriate granularity for history actions
   - Handling complex interdependencies between actions

4. **Project File Format**
   - Creating a format that is both human-readable and efficient
   - Ensuring backward compatibility for future versions
   - Balancing completeness with performance

### Next Development Steps

The following steps are planned for completing the Project module:

1. **Complete Timeline Model**
   - Implement advanced multi-track operations
   - Add comprehensive clip effect support
   - Enhance timeline event handling

2. **Enhance Serialization**
   - Complete binary serialization format
   - Add versioning and migration support
   - Implement incremental save capabilities

3. **Finalize Edit History**
   - Complete undo/redo implementation
   - Add grouping for complex operations
   - Implement history pruning for performance

4. **Develop Advanced Operations**
   - Implement complex timeline operations
   - Add transitions and effects support
   - Create timeline optimization tools

### Integration with Other Modules

While still in development, the Project module has established integration points with:

1. **CLI Module**: For project command execution
2. **Asset Module**: For managing media assets within projects
3. **Processing Module**: For rendering timeline segments

The Project module remains a key focus of current development efforts, with significant progress expected in the coming weeks as part of the completion of Phase 2 of the edv project implementation plan.
