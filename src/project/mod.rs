use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

use crate::utility::time::Duration;

pub mod rendering;
pub mod serialization;
pub mod timeline;

// Export types for convenience

/// Asset ID for resources used in projects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(Uuid);

impl AssetId {
    /// Creates a new random asset ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Returns a reference to the UUID.
    #[must_use]
    pub fn as_uuid_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for AssetId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Clip ID for timeline clips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(Uuid);

impl ClipId {
    /// Creates a new random clip ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Returns a reference to the UUID.
    #[must_use]
    pub fn as_uuid_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for ClipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ClipId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for ClipId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Asset metadata.
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    /// Duration of the asset.
    pub duration: Option<Duration>,
    /// Dimensions of the asset (width, height).
    pub dimensions: Option<(u32, u32)>,
    /// Type of asset (e.g., "video", "audio", "image").
    pub asset_type: String,
    /// Additional metadata.
    pub extra: std::collections::HashMap<String, String>,
}

/// Asset reference with path and metadata.
#[derive(Debug, Clone)]
pub struct AssetReference {
    /// ID of the asset.
    pub id: AssetId,
    /// Path to the asset file.
    pub path: PathBuf,
    /// Metadata for the asset.
    pub metadata: AssetMetadata,
}

/// Error types specific to project operations.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// Timeline error.
    #[error("Timeline error: {0}")]
    Timeline(#[from] timeline::TimelineError),

    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Asset not found.
    #[error("Asset not found: {0}")]
    AssetNotFound(AssetId),

    /// Rendering error.
    #[error("Rendering error: {0}")]
    Rendering(#[from] rendering::RenderError),
}

/// Type alias for project operation results.
pub type Result<T> = std::result::Result<T, ProjectError>;

impl From<serialization::SerializationError> for ProjectError {
    fn from(err: serialization::SerializationError) -> Self {
        ProjectError::Serialization(err.to_string())
    }
}

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
        let uuid = Uuid::from_str(s).map_err(|e| ProjectError::Serialization(e.to_string()))?;
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

/// A video editing project.
#[derive(Debug, Clone)]
pub struct Project {
    /// Name of the project.
    pub name: String,
    /// Timeline of the project.
    pub timeline: timeline::Timeline,
    /// Assets used in the project.
    pub assets: Vec<AssetReference>,
    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, String>,
    /// Project metadata
    pub project_metadata: ProjectMetadata,
}

impl Project {
    /// Creates a new project with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the project
    ///
    /// # Returns
    ///
    /// A new `Project` instance.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            timeline: timeline::Timeline::new(),
            assets: Vec::new(),
            metadata: std::collections::HashMap::new(),
            project_metadata: ProjectMetadata::new(name),
        }
    }

    /// Adds an asset to the project.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the asset file
    /// * `metadata` - Metadata for the asset
    ///
    /// # Returns
    ///
    /// The ID of the newly added asset.
    pub fn add_asset(&mut self, path: PathBuf, metadata: AssetMetadata) -> AssetId {
        let id = AssetId::new();
        let asset = AssetReference { id, path, metadata };
        self.assets.push(asset);
        self.project_metadata.update_modified();
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

    /// Gets a mutable reference to an asset by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the asset to find
    ///
    /// # Returns
    ///
    /// A mutable reference to the asset if found, or `None` if not found.
    pub fn get_asset_mut(&mut self, id: AssetId) -> Option<&mut AssetReference> {
        self.assets.iter_mut().find(|asset| asset.id == id)
    }

    /// Removes an asset from the project.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the asset to remove
    ///
    /// # Returns
    ///
    /// `Ok(())` if the asset was found and removed, or an error if not found.
    pub fn remove_asset(&mut self, id: AssetId) -> Result<()> {
        let index = self
            .assets
            .iter()
            .position(|asset| asset.id == id)
            .ok_or(ProjectError::AssetNotFound(id))?;
        self.assets.remove(index);
        self.project_metadata.update_modified();
        Ok(())
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
        self.project_metadata.clone().update_modified();
        let result = serialization::serialize_project(self, path);
        if let Err(err) = result {
            return Err(ProjectError::Serialization(err.to_string()));
        }
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
        let result = serialization::deserialize_project(path);
        if let Err(err) = result {
            return Err(ProjectError::Serialization(err.to_string()));
        }
        Ok(result.unwrap())
    }

    /// Renders the project to a video file using default settings.
    ///
    /// # Arguments
    ///
    /// * `output_path` - The path where the rendered video will be saved
    ///
    /// # Returns
    ///
    /// A `Result` containing rendering results if successful.
    pub fn render(&self, output_path: &std::path::Path) -> Result<rendering::RenderResult> {
        let config = rendering::RenderConfig::new(output_path.to_path_buf());
        self.render_with_config(config)
    }

    /// Renders the project with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The rendering configuration
    ///
    /// # Returns
    ///
    /// A `Result` containing rendering results if successful.
    pub fn render_with_config(
        &self,
        config: rendering::RenderConfig,
    ) -> Result<rendering::RenderResult> {
        rendering::render_project(self.clone(), config).map_err(ProjectError::Rendering)
    }
}

// timeline機能をエクスポート
pub use timeline::keyframes::{
    EasingFunction, KeyframeAnimation, KeyframeError, KeyframePoint, KeyframeTrack,
};
pub use timeline::{Clip, Timeline, TimelineError, Track, TrackId, TrackKind};
