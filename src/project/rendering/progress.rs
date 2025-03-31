/// Progress tracking for timeline rendering.
///
/// This module provides functionality for tracking and reporting the
/// progress of timeline rendering operations.
use crate::utility::time::{Duration, TimePosition};
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant};

/// Represents the current progress of a rendering operation.
#[derive(Debug, Clone)]
pub struct RenderProgress {
    /// Total frames to render.
    pub total_frames: u64,

    /// Frames rendered so far.
    pub frames_completed: u64,

    /// Current position in the timeline being rendered.
    pub current_position: TimePosition,

    /// Total duration of the rendering job.
    pub total_duration: Duration,

    /// Elapsed time since rendering started.
    pub elapsed: StdDuration,

    /// Estimated time remaining.
    pub estimated_remaining: Option<StdDuration>,

    /// Current frame rate of rendering (frames per second).
    pub render_fps: f64,

    /// Current stage of rendering.
    pub current_stage: RenderStage,
}

/// Stages of the rendering process.
///
/// This enum represents the different stages that occur during
/// the rendering of a timeline. It is used to track progress
/// and provide feedback to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderStage {
    /// Initial preparation stage
    Preparing,
    /// Processing assets stage
    Processing,
    /// Main rendering stage
    Rendering,
    /// Post-processing stage
    PostProcessing,
    /// Rendering complete
    Complete,
    /// Alternative name for Complete (for API compatibility)
    Completed,
    /// Rendering cancelled
    Cancelled,
}

impl RenderStage {
    /// Gets a string description of the render stage.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Preparing => "Preparing to render",
            Self::Processing => "Processing assets",
            Self::Rendering => "Rendering",
            Self::PostProcessing => "Post-processing the rendered output",
            Self::Complete => "Render complete",
            Self::Completed => "Render completed",
            Self::Cancelled => "Render cancelled",
        }
    }
}

/// Type signature for progress callback functions.
pub type ProgressCallback = Box<dyn Fn(&RenderProgress) -> bool + Send + 'static>;

/// A progress tracker for rendering operations.
pub struct ProgressTracker {
    /// The current progress state.
    progress: RenderProgress,

    /// Start time of the rendering operation.
    start_time: Instant,

    /// Time of the last progress update.
    last_update: Instant,

    /// Callback function to report progress.
    callback: Option<ProgressCallback>,

    /// Whether the render has been cancelled.
    cancelled: bool,
}

impl std::fmt::Debug for ProgressTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressTracker")
            .field("progress", &self.progress)
            .field("start_time", &self.start_time)
            .field("last_update", &self.last_update)
            .field("callback", &"<function>")
            .field("cancelled", &self.cancelled)
            .finish()
    }
}

impl ProgressTracker {
    /// Creates a new progress tracker.
    #[must_use]
    pub fn new(total_frames: u64, total_duration: Duration) -> Self {
        let now = Instant::now();
        Self {
            progress: RenderProgress {
                total_frames,
                frames_completed: 0,
                current_position: TimePosition::from_seconds(0.0),
                total_duration,
                elapsed: StdDuration::from_secs(0),
                estimated_remaining: None,
                render_fps: 0.0,
                current_stage: RenderStage::Preparing,
            },
            start_time: now,
            last_update: now,
            callback: None,
            cancelled: false,
        }
    }

    /// Sets the callback function for progress updates.
    pub fn set_callback(&mut self, callback: ProgressCallback) {
        self.callback = Some(callback);
    }

    /// Updates the progress state and calls the callback if set.
    ///
    /// Returns `true` if rendering should continue, or `false` if cancelled.
    pub fn update(&mut self, frames_completed: u64, current_position: TimePosition) -> bool {
        if self.cancelled {
            return false;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.start_time);

        // Calculate render FPS
        let time_delta = now.duration_since(self.last_update).as_secs_f64();
        let frames_delta = frames_completed.saturating_sub(self.progress.frames_completed) as f64;
        let render_fps = if time_delta > 0.0 {
            frames_delta / time_delta
        } else {
            0.0
        };

        // Calculate estimated time remaining
        let estimated_remaining =
            if frames_completed > 0 && frames_completed < self.progress.total_frames {
                let completion_ratio = frames_completed as f64 / self.progress.total_frames as f64;
                let total_estimate = elapsed.as_secs_f64() / completion_ratio;
                let remaining_secs = total_estimate - elapsed.as_secs_f64();
                if remaining_secs > 0.0 {
                    Some(StdDuration::from_secs_f64(remaining_secs))
                } else {
                    None
                }
            } else {
                None
            };

        // Update progress
        self.progress.frames_completed = frames_completed;
        self.progress.current_position = current_position;
        self.progress.elapsed = elapsed;
        self.progress.estimated_remaining = estimated_remaining;
        self.progress.render_fps = render_fps;

        self.last_update = now;

        // Call callback if set
        if let Some(callback) = &self.callback {
            if !callback(&self.progress) {
                self.cancelled = true;
                self.progress.current_stage = RenderStage::Cancelled;
                return false;
            }
        }

        true
    }

    /// Sets the current rendering stage.
    pub fn set_stage(&mut self, stage: RenderStage) {
        self.progress.current_stage = stage;

        // Call callback if stage changed
        if let Some(callback) = &self.callback {
            if !callback(&self.progress) {
                self.cancelled = true;
                self.progress.current_stage = RenderStage::Cancelled;
            }
        }
    }

    /// Marks the rendering as complete.
    pub fn complete(&mut self) {
        self.progress.frames_completed = self.progress.total_frames;
        self.progress.current_position =
            TimePosition::from_seconds(self.progress.total_duration.as_seconds());
        self.progress.estimated_remaining = None;
        self.set_stage(RenderStage::Complete);
    }

    /// Marks the rendering as failed.
    pub fn fail(&mut self) {
        self.set_stage(RenderStage::Cancelled);
    }

    /// Cancels the rendering operation.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.set_stage(RenderStage::Cancelled);
    }

    /// Returns whether the rendering has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Gets the current progress state.
    #[must_use]
    pub fn get_progress(&self) -> &RenderProgress {
        &self.progress
    }
}

/// A shared progress tracker for rendering operations.
///
/// This struct provides a thread-safe way to track rendering progress
/// and allow cancellation of the rendering process.
#[derive(Debug, Clone)]
pub struct SharedProgressTracker {
    /// Current rendering stage
    stage: std::sync::Arc<std::sync::Mutex<RenderStage>>,
    /// Whether rendering has been cancelled
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Total number of items to process
    total: std::sync::Arc<std::sync::Mutex<u64>>,
    /// Current progress value
    progress: std::sync::Arc<std::sync::Mutex<u64>>,
}

impl SharedProgressTracker {
    /// Creates a new progress tracker.
    ///
    /// # Returns
    ///
    /// A new `SharedProgressTracker` instance.
    pub fn new() -> Self {
        Self {
            stage: std::sync::Arc::new(std::sync::Mutex::new(RenderStage::Preparing)),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            total: std::sync::Arc::new(std::sync::Mutex::new(0)),
            progress: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Sets the total number of items to process.
    ///
    /// # Arguments
    ///
    /// * `total` - The total number of items
    pub fn set_total(&self, total: u64) {
        if let Ok(mut total_lock) = self.total.lock() {
            *total_lock = total;
        }
    }

    /// Increments the progress counter.
    ///
    /// # Arguments
    ///
    /// * `delta` - The amount to increment by
    pub fn increment_progress(&self, delta: u64) {
        if let Ok(mut progress_lock) = self.progress.lock() {
            *progress_lock += delta;
        }
    }

    /// Gets the current progress value.
    ///
    /// # Returns
    ///
    /// The current progress value.
    pub fn get_progress(&self) -> u64 {
        self.progress.lock().map(|progress| *progress).unwrap_or(0)
    }

    /// Gets the total number of items.
    ///
    /// # Returns
    ///
    /// The total number of items.
    pub fn get_total(&self) -> u64 {
        self.total.lock().map(|total| *total).unwrap_or(0)
    }

    /// Sets the current rendering stage.
    ///
    /// # Arguments
    ///
    /// * `stage` - The new rendering stage
    pub fn set_stage(&self, stage: RenderStage) {
        if let Ok(mut stage_lock) = self.stage.lock() {
            *stage_lock = stage;
        }
    }

    /// Gets the current rendering stage.
    ///
    /// # Returns
    ///
    /// The current rendering stage.
    pub fn get_stage(&self) -> RenderStage {
        self.stage
            .lock()
            .map(|stage| *stage)
            .unwrap_or(RenderStage::Preparing)
    }

    /// Cancels the rendering process.
    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.set_stage(RenderStage::Cancelled);
    }

    /// Checks if rendering has been cancelled.
    ///
    /// # Returns
    ///
    /// `true` if rendering has been cancelled, `false` otherwise.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for SharedProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
