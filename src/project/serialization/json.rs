/// JSON serialization for project files.
///
/// This module provides functionality for serializing and deserializing
/// project data to and from JSON format, enabling project saving and loading.
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty};

use crate::project::timeline::{Clip, Timeline, Track, TrackId, TrackKind};
use crate::project::{Project, ProjectId, ProjectMetadata};
use crate::utility::time::{Duration, TimePosition};

/// Error types for JSON serialization operations.
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    /// Error during I/O operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error during JSON serialization or deserialization.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Error when the file format is incompatible.
    #[error("Incompatible file format: {0}")]
    IncompatibleFormat(String),

    /// Error when the project version is unsupported.
    #[error("Unsupported project version: {0}")]
    UnsupportedVersion(String),

    /// Error from timeline operations
    #[error("Timeline error: {0}")]
    Timeline(#[from] crate::project::timeline::TimelineError),
}

/// Type alias for serialization operation results.
pub type Result<T> = std::result::Result<T, SerializationError>;

/// Current version of the project file format.
const CURRENT_VERSION: &str = "1.0.0";

/// Metadata included in the project file.
#[derive(Debug, Serialize, Deserialize)]
struct ProjectFileMetadata {
    /// Version of the project file format.
    version: String,

    /// Type of the project file.
    file_type: String,

    /// Application that created the project.
    created_by: String,
}

/// Wrapper for the entire project file.
#[derive(Debug, Serialize, Deserialize)]
struct ProjectFile {
    /// Metadata about the project file.
    metadata: ProjectFileMetadata,

    /// The actual project data.
    project: SerializedProject,
}

/// Serializable representation of a project.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedProject {
    /// Unique identifier for the project.
    id: String,

    /// Project metadata.
    metadata: SerializedProjectMetadata,

    /// Timeline data.
    timeline: SerializedTimeline,

    /// Assets used in the project.
    assets: Vec<SerializedAssetReference>,
}

/// Serializable representation of project metadata.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedProjectMetadata {
    /// Name of the project.
    name: String,

    /// Creation timestamp.
    created_at: String,

    /// Last modification timestamp.
    modified_at: String,

    /// Project description.
    description: String,

    /// Tags associated with the project.
    tags: Vec<String>,
}

/// Serializable representation of a timeline.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedTimeline {
    /// Tracks in the timeline.
    tracks: Vec<SerializedTrack>,

    /// Timeline duration in seconds.
    duration: f64,
}

/// Serializable representation of a track.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedTrack {
    /// Unique identifier for the track.
    id: String,

    /// Type of the track.
    kind: String,

    /// Name of the track.
    name: String,

    /// Clips in the track.
    clips: Vec<SerializedClip>,

    /// Whether the track is muted.
    muted: bool,

    /// Whether the track is locked for editing.
    locked: bool,
}

/// Serializable representation of a clip.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedClip {
    /// Unique identifier for the clip.
    id: String,

    /// ID of the asset used in the clip.
    asset_id: String,

    /// Position of the clip in the timeline.
    position: f64,

    /// Duration of the clip.
    duration: f64,

    /// Start position in the source asset.
    source_start: f64,

    /// End position in the source asset.
    source_end: f64,
}

/// Serializable representation of an asset reference.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedAssetReference {
    /// Unique identifier for the asset.
    id: String,

    /// Path to the asset file.
    path: String,

    /// Metadata for the asset.
    metadata: SerializedAssetMetadata,
}

/// Serializable representation of asset metadata.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedAssetMetadata {
    /// Duration of the asset in seconds.
    duration: Option<f64>,

    /// Dimensions of the asset as [width, height].
    dimensions: Option<[u32; 2]>,

    /// Type of the asset.
    asset_type: String,

    /// Additional metadata as key-value pairs.
    extra: std::collections::HashMap<String, String>,
}

/// Serializes a project to JSON and writes it to a file.
///
/// # Arguments
///
/// * `project` - The project to serialize
/// * `path` - The path where the serialized project will be saved
///
/// # Returns
///
/// A `Result` containing `()` if the serialization was successful,
/// or an error if the operation failed.
///
/// # Errors
///
/// Returns an error if:
/// * The file could not be created or written to
/// * The project could not be serialized to JSON
pub fn serialize_project(project: &Project, path: &Path) -> Result<()> {
    // Create the file
    let file = File::create(path)?;
    let writer = BufWriter::new(file);

    // Create the serializable project
    let serialized_project = convert_to_serialized_project(project);

    // Create the project file wrapper
    let project_file = ProjectFile {
        metadata: ProjectFileMetadata {
            version: CURRENT_VERSION.to_string(),
            file_type: "edv_project".to_string(),
            created_by: format!("edv {}", env!("CARGO_PKG_VERSION")),
        },
        project: serialized_project,
    };

    // Write the project file to JSON
    to_writer_pretty(writer, &project_file)?;

    Ok(())
}

/// Deserializes a project from a JSON file.
///
/// # Arguments
///
/// * `path` - The path to the serialized project file
///
/// # Returns
///
/// A `Result` containing the deserialized `Project` if successful,
/// or an error if the operation failed.
///
/// # Errors
///
/// Returns an error if:
/// * The file could not be read
/// * The JSON could not be parsed
/// * The file format is incompatible
/// * The project version is unsupported
pub fn deserialize_project(path: &Path) -> Result<Project> {
    // Open the file
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Parse the JSON
    let project_file: ProjectFile = from_reader(reader)?;

    // Check the version
    if !is_version_compatible(&project_file.metadata.version) {
        return Err(SerializationError::UnsupportedVersion(
            project_file.metadata.version,
        ));
    }

    // Check the file type
    if project_file.metadata.file_type != "edv_project" {
        return Err(SerializationError::IncompatibleFormat(
            project_file.metadata.file_type,
        ));
    }

    // Convert to project
    let project = convert_from_serialized_project(&project_file.project)?;

    Ok(project)
}

/// Converts a `Project` to its serializable representation.
fn convert_to_serialized_project(project: &Project) -> SerializedProject {
    // Convert project metadata
    let serialized_metadata = SerializedProjectMetadata {
        name: project.metadata.name.clone(),
        created_at: project.metadata.created_at.to_rfc3339(),
        modified_at: project.metadata.modified_at.to_rfc3339(),
        description: project.metadata.description.clone(),
        tags: project.metadata.tags.clone(),
    };

    // Convert timeline
    let serialized_timeline = convert_to_serialized_timeline(&project.timeline);

    // Convert assets
    let serialized_assets = project
        .assets
        .iter()
        .map(convert_to_serialized_asset_reference)
        .collect();

    SerializedProject {
        id: project.id.to_string(),
        metadata: serialized_metadata,
        timeline: serialized_timeline,
        assets: serialized_assets,
    }
}

/// Converts a `Timeline` to its serializable representation.
fn convert_to_serialized_timeline(timeline: &Timeline) -> SerializedTimeline {
    let serialized_tracks = timeline
        .get_tracks()
        .iter()
        .map(convert_to_serialized_track)
        .collect();

    SerializedTimeline {
        tracks: serialized_tracks,
        duration: timeline.duration().as_seconds(),
    }
}

/// Converts a `Track` to its serializable representation.
fn convert_to_serialized_track(track: &Track) -> SerializedTrack {
    let serialized_clips = track
        .get_clips()
        .iter()
        .map(convert_to_serialized_clip)
        .collect();

    SerializedTrack {
        id: track.id().to_string(),
        kind: track.kind().to_string(),
        name: track.name().to_string(),
        clips: serialized_clips,
        muted: track.is_muted(),
        locked: track.is_locked(),
    }
}

/// Converts a `Clip` to its serializable representation.
fn convert_to_serialized_clip(clip: &Clip) -> SerializedClip {
    SerializedClip {
        id: clip.id().to_string(),
        asset_id: clip.asset_id().to_string(),
        position: clip.position().as_seconds(),
        duration: clip.duration().as_seconds(),
        source_start: clip.source_start().as_seconds(),
        source_end: clip.source_end().as_seconds(),
    }
}

/// Converts an asset reference to its serializable representation.
fn convert_to_serialized_asset_reference(
    asset_ref: &crate::project::AssetReference,
) -> SerializedAssetReference {
    SerializedAssetReference {
        id: asset_ref.id.to_string(),
        path: asset_ref.path.to_string_lossy().to_string(),
        metadata: SerializedAssetMetadata {
            duration: asset_ref.metadata.duration.map(|d| d.as_seconds()),
            dimensions: asset_ref
                .metadata
                .dimensions
                .map(|(width, height)| [width, height]),
            asset_type: asset_ref.metadata.asset_type.clone(),
            extra: asset_ref.metadata.extra.clone(),
        },
    }
}

/// Converts a serialized project to a `Project`.
fn convert_from_serialized_project(serialized: &SerializedProject) -> Result<Project> {
    // Convert project ID
    let id = ProjectId::from_string(&serialized.id)
        .map_err(|e| SerializationError::IncompatibleFormat(e.to_string()))?;

    // Convert project metadata
    let metadata = convert_from_serialized_project_metadata(&serialized.metadata)?;

    // Convert timeline
    let timeline = convert_from_serialized_timeline(&serialized.timeline)?;

    // Convert assets
    let assets = serialized
        .assets
        .iter()
        .map(convert_from_serialized_asset_reference)
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Create the project
    Ok(Project::from_components(id, metadata, timeline, assets))
}

/// Converts serialized project metadata to `ProjectMetadata`.
fn convert_from_serialized_project_metadata(
    serialized: &SerializedProjectMetadata,
) -> Result<ProjectMetadata> {
    // Parse timestamps
    let created_at = chrono::DateTime::parse_from_rfc3339(&serialized.created_at)
        .map_err(|e| SerializationError::IncompatibleFormat(e.to_string()))?
        .with_timezone(&chrono::Utc);

    let modified_at = chrono::DateTime::parse_from_rfc3339(&serialized.modified_at)
        .map_err(|e| SerializationError::IncompatibleFormat(e.to_string()))?
        .with_timezone(&chrono::Utc);

    Ok(ProjectMetadata {
        name: serialized.name.clone(),
        created_at,
        modified_at,
        description: serialized.description.clone(),
        tags: serialized.tags.clone(),
    })
}

/// Converts a serialized timeline to a `Timeline`.
fn convert_from_serialized_timeline(serialized: &SerializedTimeline) -> Result<Timeline> {
    let mut timeline = Timeline::new();

    // Add each track
    for serialized_track in &serialized.tracks {
        let track_id = convert_from_serialized_track(serialized_track, &mut timeline)?;

        // Add clips to the track
        for serialized_clip in &serialized_track.clips {
            let clip = convert_from_serialized_clip(serialized_clip)?;
            timeline.add_clip(track_id, clip)?;
        }
    }

    Ok(timeline)
}

/// Converts a serialized track to a `Track` and adds it to the timeline.
fn convert_from_serialized_track(
    serialized: &SerializedTrack,
    timeline: &mut Timeline,
) -> Result<TrackId> {
    // Parse track kind
    let kind = match serialized.kind.as_str() {
        "Video" => TrackKind::Video,
        "Audio" => TrackKind::Audio,
        "Subtitle" => TrackKind::Subtitle,
        unknown => {
            return Err(SerializationError::IncompatibleFormat(format!(
                "Unknown track kind: {unknown}"
            )));
        }
    };

    // Add the track to the timeline
    let track_id = timeline.add_track(kind);

    // Set track properties
    let track = timeline
        .get_track_mut(track_id)
        .ok_or_else(|| SerializationError::IncompatibleFormat("Failed to add track".to_string()))?;

    track.set_name(&serialized.name);
    track.set_muted(serialized.muted);
    track.set_locked(serialized.locked);

    Ok(track_id)
}

/// Converts a serialized clip to a `Clip`.
fn convert_from_serialized_clip(serialized: &SerializedClip) -> Result<Clip> {
    // Parse clip ID
    let id = serialized
        .id
        .parse()
        .map_err(|e| SerializationError::IncompatibleFormat(format!("Invalid clip ID: {e}")))?;

    // Parse asset ID
    let asset_id = serialized
        .asset_id
        .parse()
        .map_err(|e| SerializationError::IncompatibleFormat(format!("Invalid asset ID: {e}")))?;

    // Create the clip
    Ok(Clip::new(
        id,
        asset_id,
        TimePosition::from_seconds(serialized.position),
        Duration::from_seconds(serialized.duration),
        TimePosition::from_seconds(serialized.source_start),
        TimePosition::from_seconds(serialized.source_end),
    ))
}

/// Converts a serialized asset reference to an `AssetReference`.
fn convert_from_serialized_asset_reference(
    serialized: &SerializedAssetReference,
) -> Result<crate::project::AssetReference> {
    // Parse asset ID
    let id = serialized
        .id
        .parse()
        .map_err(|e| SerializationError::IncompatibleFormat(format!("Invalid asset ID: {e}")))?;

    // Create asset metadata
    let metadata = crate::project::AssetMetadata {
        duration: serialized.metadata.duration.map(Duration::from_seconds),
        dimensions: serialized.metadata.dimensions.map(|[w, h]| (w, h)),
        asset_type: serialized.metadata.asset_type.clone(),
        extra: serialized.metadata.extra.clone(),
    };

    // Create the asset reference
    Ok(crate::project::AssetReference {
        id,
        path: std::path::PathBuf::from(&serialized.path),
        metadata,
    })
}

/// Checks if the given version is compatible with the current implementation.
fn is_version_compatible(version: &str) -> bool {
    // Parse the version
    let version_parts: Vec<&str> = version.split('.').collect();
    if version_parts.len() != 3 {
        return false;
    }

    // Parse the major version
    let major = version_parts[0].parse::<u32>().unwrap_or(0);

    // Currently, we only support version 1.x.x
    major == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use tempfile::NamedTempFile;

    // Helper to create a test project
    fn create_test_project() -> Project {
        let mut project = Project::new("Test Project");

        // Add a video track
        let video_track_id = project.timeline.add_track(TrackKind::Video);
        let video_track = project.timeline.get_track_mut(video_track_id).unwrap();
        video_track.set_name("Video Track");

        // Add an audio track
        let audio_track_id = project.timeline.add_track(TrackKind::Audio);
        let audio_track = project.timeline.get_track_mut(audio_track_id).unwrap();
        audio_track.set_name("Audio Track");

        // Add a test asset
        let asset_id = crate::project::AssetId::new();
        let asset_ref = crate::project::AssetReference {
            id: asset_id,
            path: std::path::PathBuf::from("/path/to/test/video.mp4"),
            metadata: crate::project::AssetMetadata {
                duration: Some(Duration::from_seconds(30.0)),
                dimensions: Some((1920, 1080)),
                asset_type: "video".to_string(),
                extra: std::collections::HashMap::new(),
            },
        };
        project.assets.push(asset_ref);

        // Add a clip to the video track
        let clip = Clip::new(
            crate::project::ClipId::new(),
            asset_id,
            TimePosition::from_seconds(0.0),
            Duration::from_seconds(10.0),
            TimePosition::from_seconds(5.0),
            TimePosition::from_seconds(15.0),
        );
        project.timeline.add_clip(video_track_id, clip).unwrap();

        project
    }

    #[test]
    fn test_serialization_and_deserialization() {
        let project = create_test_project();

        // Create a temporary file for serialization
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Serialize the project
        let result = serialize_project(&project, temp_path);
        assert!(result.is_ok());

        // Deserialize the project
        let deserialized = deserialize_project(temp_path);
        assert!(deserialized.is_ok());

        let deserialized_project = deserialized.unwrap();

        // Verify project ID
        assert_eq!(project.id, deserialized_project.id);

        // Verify project metadata
        assert_eq!(project.metadata.name, deserialized_project.metadata.name);
        assert_eq!(
            project.metadata.description,
            deserialized_project.metadata.description
        );

        // Verify timeline structure
        assert_eq!(
            project.timeline.get_tracks().len(),
            deserialized_project.timeline.get_tracks().len()
        );

        // Verify assets
        assert_eq!(project.assets.len(), deserialized_project.assets.len());
        assert_eq!(project.assets[0].id, deserialized_project.assets[0].id);
    }

    #[test]
    fn test_incompatible_version() {
        let project = create_test_project();

        // Create a temporary file for serialization
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Serialize the project
        let result = serialize_project(&project, temp_path);
        assert!(result.is_ok());

        // Modify the file to change the version
        let mut file_content = String::new();
        {
            let mut file = File::open(temp_path).unwrap();
            file.read_to_string(&mut file_content).unwrap();
        }

        let modified_content =
            file_content.replace("\"version\": \"1.0.0\"", "\"version\": \"2.0.0\"");

        {
            let mut file = File::create(temp_path).unwrap();
            file.write_all(modified_content.as_bytes()).unwrap();
        }

        // Try to deserialize with incompatible version
        let result = deserialize_project(temp_path);
        assert!(matches!(
            result,
            Err(SerializationError::UnsupportedVersion(_))
        ));
    }
}
