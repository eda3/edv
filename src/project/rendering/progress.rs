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

/// Represents the different stages of the rendering process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStage {
    /// Preparing the timeline data for rendering.
    Preparing,

    /// Rendering video frames.
    RenderingVideo,

    /// Processing audio tracks.
    ProcessingAudio,

    /// Muxing audio and video streams together.
    Muxing,

    /// Finalizing the output file.
    Finalizing,

    /// Rendering is complete.
    Complete,

    /// Rendering failed.
    Failed,

    /// Rendering was cancelled by the user.
    Cancelled,
}

impl RenderStage {
    /// Gets a string description of the render stage.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Preparing => "Preparing timeline data",
            Self::RenderingVideo => "Rendering video frames",
            Self::ProcessingAudio => "Processing audio tracks",
            Self::Muxing => "Combining audio and video",
            Self::Finalizing => "Finalizing output file",
            Self::Complete => "Render complete",
            Self::Failed => "Render failed",
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
        self.set_stage(RenderStage::Failed);
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

/// A shareable progress tracker for use across threads.
#[derive(Debug, Clone)]
pub struct SharedProgressTracker {
    /// Inner progress tracker wrapped in Arc<Mutex>.
    inner: Arc<Mutex<ProgressTracker>>,
}

impl SharedProgressTracker {
    /// Creates a new shared progress tracker.
    #[must_use]
    pub fn new(total_frames: u64, total_duration: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressTracker::new(
                total_frames,
                total_duration,
            ))),
        }
    }

    /// Sets the callback function for progress updates.
    pub fn set_callback(&self, callback: ProgressCallback) {
        if let Ok(mut tracker) = self.inner.lock() {
            tracker.set_callback(callback);
        }
    }

    /// Updates the progress state.
    ///
    /// Returns `true` if rendering should continue, or `false` if cancelled.
    pub fn update(&self, frames_completed: u64, current_position: TimePosition) -> bool {
        self.inner
            .lock()
            .map(|mut tracker| tracker.update(frames_completed, current_position))
            .unwrap_or(false)
    }

    /// Sets the current rendering stage.
    pub fn set_stage(&self, stage: RenderStage) {
        if let Ok(mut tracker) = self.inner.lock() {
            tracker.set_stage(stage);
        }
    }

    /// Marks the rendering as complete.
    pub fn complete(&self) {
        if let Ok(mut tracker) = self.inner.lock() {
            tracker.complete();
        }
    }

    /// Marks the rendering as failed.
    pub fn fail(&self) {
        if let Ok(mut tracker) = self.inner.lock() {
            tracker.fail();
        }
    }

    /// Cancels the rendering operation.
    pub fn cancel(&self) {
        if let Ok(mut tracker) = self.inner.lock() {
            tracker.cancel();
        }
    }

    /// Returns whether the rendering has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.inner
            .lock()
            .map(|tracker| tracker.is_cancelled())
            .unwrap_or(true)
    }

    /// Gets the current progress state.
    pub fn get_progress(&self) -> Option<RenderProgress> {
        self.inner
            .lock()
            .map(|tracker| tracker.get_progress().clone())
            .ok()
    }
}
