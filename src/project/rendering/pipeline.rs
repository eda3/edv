/// Rendering pipeline for timeline projects.
///
/// This module provides the main pipeline for rendering timeline projects
/// to video files, coordinating the various stages of the rendering process.
use std::path::Path;
use std::thread;

use crate::project::Project;
use crate::project::rendering::RenderError;
use crate::project::rendering::compositor::TrackCompositor;
use crate::project::rendering::config::RenderConfig;
use crate::project::rendering::progress::{
    ProgressCallback, RenderProgress, SharedProgressTracker,
};
use crate::utility::time::Duration;

/// Result of a rendering operation.
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// Path to the rendered output file.
    pub output_path: std::path::PathBuf,

    /// Duration of the rendered video.
    pub duration: Duration,

    /// Total frames rendered.
    pub total_frames: u64,

    /// Time taken to render.
    pub render_time: std::time::Duration,

    /// Average rendering speed (frames per second).
    pub average_render_fps: f64,
}

/// Manages the rendering pipeline for timeline projects.
#[derive(Debug)]
pub struct RenderPipeline {
    /// The project being rendered.
    project: Project,

    /// Rendering configuration.
    config: RenderConfig,

    /// Progress tracker for the rendering process.
    progress: SharedProgressTracker,

    /// Start time of the rendering process.
    start_time: Option<std::time::Instant>,
}

impl RenderPipeline {
    /// Creates a new render pipeline.
    ///
    /// # Arguments
    ///
    /// * `project` - The project to render
    /// * `config` - The rendering configuration
    ///
    /// # Returns
    ///
    /// A new `RenderPipeline` instance.
    #[must_use]
    pub fn new(project: Project, config: RenderConfig) -> Self {
        // Calculate total frames based on timeline duration and frame rate
        let timeline_duration = Self::calculate_timeline_duration(&project);
        let total_frames = (timeline_duration.as_seconds() * config.frame_rate) as u64;

        Self {
            project,
            config,
            progress: SharedProgressTracker::new(total_frames, timeline_duration),
            start_time: None,
        }
    }

    /// Calculates the duration of the timeline in the project.
    fn calculate_timeline_duration(project: &Project) -> Duration {
        // Find the max end time of all clips across all tracks
        let mut max_duration = Duration::from_seconds(0.0);

        for track in project.timeline.get_tracks() {
            for clip in track.get_clips() {
                let clip_end = clip.position() + clip.duration();
                if clip_end.as_seconds() > max_duration.as_seconds() {
                    max_duration = Duration::from_seconds(clip_end.as_seconds());
                }
            }
        }

        max_duration
    }

    /// Sets the progress callback for the rendering process.
    ///
    /// # Arguments
    ///
    /// * `callback` - The callback function to call with progress updates
    pub fn set_progress_callback(&self, callback: ProgressCallback) {
        self.progress.set_callback(callback);
    }

    /// Renders the project synchronously.
    ///
    /// # Returns
    ///
    /// A `Result` containing rendering results on success, or an error if rendering failed.
    pub fn render(&mut self) -> Result<RenderResult, RenderError> {
        self.start_time = Some(std::time::Instant::now());

        // Validate output path
        if self.config.output_path.as_os_str().is_empty() {
            return Err(RenderError::Timeline("Output path is empty".to_string()));
        }

        // Ensure the output directory exists
        if let Some(parent) = self.config.output_path.parent() {
            std::fs::create_dir_all(parent).map_err(RenderError::from)?;
        }

        // Create compositor
        let mut compositor =
            TrackCompositor::new(self.project.timeline.clone(), self.project.assets.clone());

        // Set progress tracker
        compositor.set_progress_tracker(self.progress.clone());

        // Execute the composition
        compositor.compose(&self.config)?;

        // Calculate results
        let end_time = std::time::Instant::now();
        let render_time = end_time.duration_since(self.start_time.unwrap());

        let progress = self.progress.get_progress().ok_or_else(|| {
            RenderError::Timeline("Failed to get progress information".to_string())
        })?;

        let render_result = RenderResult {
            output_path: self.config.output_path.clone(),
            duration: progress.total_duration,
            total_frames: progress.total_frames,
            render_time,
            average_render_fps: progress.total_frames as f64 / render_time.as_secs_f64(),
        };

        Ok(render_result)
    }

    /// Renders the project asynchronously in a separate thread.
    ///
    /// # Arguments
    ///
    /// * `callback` - Optional callback function to be called with the result
    ///
    /// # Returns
    ///
    /// A handle to the rendering thread.
    pub fn render_async<F>(
        self,
        callback: Option<F>,
    ) -> thread::JoinHandle<Result<RenderResult, RenderError>>
    where
        F: FnOnce(Result<RenderResult, RenderError>) + Send + 'static,
    {
        thread::spawn(move || {
            let mut pipeline = self;
            let result = pipeline.render();

            // コールバックがあれば実行
            if let Some(cb) = callback {
                match &result {
                    Ok(r) => cb(Ok(r.clone())),   // 成功した場合は結果のクローンを渡す
                    Err(e) => cb(Err(e.clone())), // エラーの場合はエラーのクローンを渡す
                }
            }

            result
        })
    }

    /// Cancels the rendering process.
    pub fn cancel(&self) {
        self.progress.cancel();
    }

    /// Checks if the rendering process has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.progress.is_cancelled()
    }

    /// Gets the current rendering progress.
    #[must_use]
    pub fn get_progress(&self) -> Option<RenderProgress> {
        self.progress.get_progress()
    }
}

/// Renders a project with the provided configuration.
///
/// This is a convenience function for simple rendering scenarios.
///
/// # Arguments
///
/// * `project` - The project to render
/// * `config` - The rendering configuration
///
/// # Returns
///
/// A `Result` containing rendering results on success, or an error if rendering failed.
pub fn render_project(project: Project, config: RenderConfig) -> Result<RenderResult, RenderError> {
    let mut pipeline = RenderPipeline::new(project, config);
    pipeline.render()
}

/// Renders a project to the specified output path with default settings.
///
/// This is a convenience function for quick rendering with default settings.
///
/// # Arguments
///
/// * `project` - The project to render
/// * `output_path` - The path to save the rendered video
///
/// # Returns
///
/// A `Result` containing rendering results on success, or an error if rendering failed.
pub fn render_project_simple(
    project: Project,
    output_path: &Path,
) -> Result<RenderResult, RenderError> {
    let config = RenderConfig::new(output_path.to_path_buf());
    render_project(project, config)
}
