use chrono::{DateTime, Utc};
/// Project management functionality.
///
/// This module provides functionality for creating, editing, and managing
/// video editing projects, including timeline editing, asset management,
/// and project serialization/deserialization.
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

pub mod serialization;
pub mod timeline;

/// Error types specific to project operations.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// Error when operating on a timeline.
    #[error("Timeline error: {0}")]
    Timeline(#[from] timeline::TimelineError),

    /// Error during serialization or deserialization.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serialization::SerializationError),

    /// Error when parsing an ID.
    #[error("Invalid ID format: {0}")]
    InvalidId(String),

    /// Error when an asset is not found.
    #[error("Asset not found: {0}")]
    AssetNotFound(AssetId),

    /// Error during file operations.
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
}

/// Type alias for project operation results.
pub type Result<T> = std::result::Result<T, ProjectError>;

/// Unique identifier for a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectId(Uuid);

impl ProjectId {
    /// Creates a new random project ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a project ID from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid UUID.
    pub fn from_string(s: &str) -> Result<Self> {
        let uuid = Uuid::from_str(s).map_err(|e| ProjectError::InvalidId(e.to_string()))?;
        Ok(Self(uuid))
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(Uuid);

impl ClipId {
    /// Creates a new random clip ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for ClipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ClipId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

impl Default for ClipId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(Uuid);

impl AssetId {
    /// Creates a new random asset ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AssetId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata for a project.
#[derive(Debug, Clone)]
pub struct ProjectMetadata {
    /// Name of the project.
    pub name: String,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp.
    pub modified_at: DateTime<Utc>,

    /// Project description.
    pub description: String,

    /// Tags associated with the project.
    pub tags: Vec<String>,
}

impl ProjectMetadata {
    /// Creates new project metadata with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the project
    #[must_use]
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            name: name.to_string(),
            created_at: now,
            modified_at: now,
            description: String::new(),
            tags: Vec::new(),
        }
    }

    /// Updates the modification timestamp to the current time.
    pub fn update_modified(&mut self) {
        self.modified_at = Utc::now();
    }
}

/// Metadata for an asset.
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    /// Duration of the asset.
    pub duration: Option<crate::utility::time::Duration>,

    /// Dimensions of the asset (width, height).
    pub dimensions: Option<(u32, u32)>,

    /// Type of the asset (e.g., "video", "audio", "image").
    pub asset_type: String,

    /// Additional metadata as key-value pairs.
    pub extra: HashMap<String, String>,
}

/// Reference to an asset used in a project.
#[derive(Debug, Clone)]
pub struct AssetReference {
    /// Unique identifier for the asset.
    pub id: AssetId,

    /// Path to the asset file.
    pub path: PathBuf,

    /// Metadata for the asset.
    pub metadata: AssetMetadata,
}

/// A video editing project.
#[derive(Debug, Clone)]
pub struct Project {
    /// Unique identifier for the project.
    pub id: ProjectId,

    /// Project metadata.
    pub metadata: ProjectMetadata,

    /// Timeline for the project.
    pub timeline: timeline::Timeline,

    /// Assets used in the project.
    pub assets: Vec<AssetReference>,
}

impl Project {
    /// Creates a new empty project with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the project
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            id: ProjectId::new(),
            metadata: ProjectMetadata::new(name),
            timeline: timeline::Timeline::new(),
            assets: Vec::new(),
        }
    }

    /// Creates a project from its components.
    ///
    /// # Arguments
    ///
    /// * `id` - The project ID
    /// * `metadata` - The project metadata
    /// * `timeline` - The project timeline
    /// * `assets` - The project assets
    #[must_use]
    pub fn from_components(
        id: ProjectId,
        metadata: ProjectMetadata,
        timeline: timeline::Timeline,
        assets: Vec<AssetReference>,
    ) -> Self {
        Self {
            id,
            metadata,
            timeline,
            assets,
        }
    }

    /// Adds an asset to the project.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the asset file
    /// * `metadata` - The metadata for the asset
    ///
    /// # Returns
    ///
    /// The ID of the added asset.
    pub fn add_asset(&mut self, path: PathBuf, metadata: AssetMetadata) -> AssetId {
        let id = AssetId::new();
        self.assets.push(AssetReference { id, path, metadata });
        self.metadata.update_modified();
        id
    }

    /// Gets an asset by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the asset to find
    ///
    /// # Returns
    ///
    /// A reference to the asset if found, or `None` if not found.
    pub fn get_asset(&self, id: AssetId) -> Option<&AssetReference> {
        self.assets.iter().find(|asset| asset.id == id)
    }

    /// Removes an asset from the project.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the asset to remove
    ///
    /// # Returns
    ///
    /// `true` if the asset was found and removed, `false` otherwise.
    pub fn remove_asset(&mut self, id: AssetId) -> bool {
        let len = self.assets.len();
        self.assets.retain(|asset| asset.id != id);
        let removed = self.assets.len() < len;
        if removed {
            self.metadata.update_modified();
        }
        removed
    }

    /// Saves the project to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the project will be saved
    ///
    /// # Errors
    ///
    /// Returns an error if the project could not be saved.
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        self.metadata.clone().update_modified();
        serialization::serialize_project(self, path)?;
        Ok(())
    }

    /// Loads a project from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the project file
    ///
    /// # Returns
    ///
    /// The loaded project if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the project could not be loaded.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let project = serialization::deserialize_project(path)?;
        Ok(project)
    }
}
