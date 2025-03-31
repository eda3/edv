//! Rendering module for video composition and processing.
//!
//! This module provides functionality for rendering video timelines,
//! including composition of multiple tracks, applying effects,
//! and handling audio/video synchronization.

/// Rendering functionality for projects.
///
/// This module provides the functionality for rendering projects to video files,
/// including configuration, progress tracking, and background rendering.
pub mod cache;
pub mod compositor;
pub mod config;
pub mod error;
pub mod gpu_accelerator;
pub mod pipeline;
pub mod progress;

pub use cache::{CacheEntry, CacheMetadata, RenderCache};
pub use compositor::{CompositionError, TrackCompositor};
pub use config::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
pub use error::{RenderError, Result};
pub use gpu_accelerator::{GpuAccelerator, create_gpu_accelerator, has_gpu_acceleration};
pub use pipeline::{RenderPipeline, RenderResult, render_project, render_project_simple};
pub use progress::{ProgressCallback, RenderProgress, RenderStage, SharedProgressTracker};
