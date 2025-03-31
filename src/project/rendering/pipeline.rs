/// Rendering pipeline for timeline projects.
///
/// This module provides the main pipeline for rendering timeline projects
/// to video files, coordinating the various stages of the rendering process.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crate::project::Project;
use crate::project::rendering::compositor::TrackCompositor;
use crate::project::rendering::config::RenderConfig;
use crate::project::rendering::progress::{
    ProgressCallback, RenderProgress, RenderStage, SharedProgressTracker,
};
use crate::project::rendering::{RenderCache, RenderError};
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

    /// Whether the result was loaded from cache.
    pub from_cache: bool,
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

    /// Cache for rendered assets.
    cache: Option<Arc<RenderCache>>,

    /// Whether the pipeline is currently in auto-loading mode.
    auto_loading: bool,
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
            progress: SharedProgressTracker::new(),
            start_time: None,
            cache: None,
            auto_loading: false,
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
    pub fn set_progress_callback(&self, _callback: ProgressCallback) {
        // This method is retained for API compatibility but no longer does anything
        // since the new SharedProgressTracker doesn't support callbacks
    }

    /// Initializes the render cache.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory for cache files
    /// * `max_size` - Maximum size of the cache in bytes
    ///
    /// # Returns
    ///
    /// `Ok(())` if the cache was initialized, or an error if initialization failed.
    pub fn init_cache(
        &mut self,
        cache_dir: PathBuf,
        max_size: Option<u64>,
    ) -> Result<(), RenderError> {
        // Determine cache directory - use config cache_dir, provided cache_dir or core cache_dir
        let cache_directory = self.config.cache_dir.clone().unwrap_or(cache_dir);

        // Determine max cache size from config or provided value
        let max_cache_size = self.config.max_cache_size.or(max_size);

        // Create render cache
        let render_cache = RenderCache::new(cache_directory, max_cache_size)
            .map_err(|e| RenderError::Cache(format!("Failed to initialize cache: {}", e)))?;

        self.cache = Some(Arc::new(render_cache));

        Ok(())
    }

    /// Auto-loads assets when the project is loaded.
    ///
    /// This renders all assets in the project at load time,
    /// so they're immediately available for preview and editing.
    ///
    /// # Returns
    ///
    /// `Ok(())` if assets were loaded successfully, or an error if loading failed.
    ///
    /// # Errors
    ///
    /// Returns an error if asset rendering fails or if cache operations fail.
    pub fn auto_load_assets(&mut self) -> Result<(), RenderError> {
        if !self.config.use_cache || !self.config.auto_load_assets {
            return Ok(());
        }

        // キャッシュが設定されていない場合も早期リターン
        let cache = match &self.cache {
            Some(cache) => cache,
            None => return Ok(()),
        };

        // マークauto_loadingをtrueに設定してレンダリングをキャッシュに保存
        self.auto_loading = true;
        self.progress.set_stage(RenderStage::Preparing);

        // レンダリング設定のパラメータハッシュを計算（一度だけ）
        let params_hash = cache.hash_params(&self.config);

        // 処理が必要なアセットをフィルタリング
        let assets_to_process: Vec<_> = self
            .project
            .assets
            .iter()
            .filter(|asset| {
                // ビデオまたはオーディオアセットのみを処理
                let is_media = matches!(asset.metadata.asset_type.as_str(), "video" | "audio");

                // すでにキャッシュされているものはスキップ
                let cached = cache.get(asset.id, params_hash).is_some();

                is_media && !cached
            })
            .collect();

        // 処理するものがなければ早期リターン
        if assets_to_process.is_empty() {
            return Ok(());
        }

        self.progress.set_stage(RenderStage::Processing);
        self.progress.set_total(assets_to_process.len() as u64);

        // rayonを使用して並列処理
        use rayon::prelude::*;
        use std::sync::{Arc, Mutex};

        let errors = Mutex::new(Vec::new());
        let progress = Arc::new(self.progress.clone());
        let cache_ref = Arc::clone(cache);

        assets_to_process.par_iter().for_each(|asset| {
            // 処理がキャンセルされた場合はスキップ
            if progress.is_cancelled() {
                return;
            }

            let result = || -> Result<(), String> {
                let asset_id = asset.id;

                // アセットパスから一時ファイルパスを生成
                let temp_file = std::env::temp_dir().join(format!("autoload_{}.mp4", asset_id));

                // FFmpegを使用してアセットを変換（実際の処理）
                // 注: 実際の実装はここで行われますが、この例では省略

                // キャッシュに追加
                // 注: 実際の実装では、ここでキャッシュを更新します

                Ok(())
            }();

            // エラーがあれば記録
            if let Err(e) = result {
                let mut error_list = errors.lock().unwrap();
                error_list.push(format!("Failed to process asset {}: {}", asset.id, e));
            }

            // 進捗を更新
            progress.increment_progress(1);
        });

        // エラーチェック
        let error_list = errors.into_inner().unwrap();
        if !error_list.is_empty() {
            return Err(RenderError::ProcessingFailed(error_list.join("; ")));
        }

        self.progress.set_stage(RenderStage::Completed);
        Ok(())
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

        // キャッシュが設定され、使用する場合はプロジェクト全体のキャッシュをチェック
        if self.config.use_cache && self.cache.is_some() {
            let cache = self.cache.as_ref().unwrap();

            // プロジェクト全体のハッシュを計算（簡略化のためproject_id + 固定値を使用）
            let project_hash = 12345; // 実際の実装では、プロジェクト構造から適切なハッシュを計算

            // すべてのアセットIDを取得（これは実際のアセットIDではなく、ダミー）
            let dummy_asset_id = crate::project::AssetId::new();

            // キャッシュをチェック
            if let Some(entry) = cache.get(dummy_asset_id, project_hash) {
                // キャッシュから読み込めた場合は、そのまま返す
                // 実際の実装では、キャッシュされたレンダリング結果を検証

                // キャッシュから取得した場合も進捗を更新
                self.progress.set_stage(RenderStage::Complete);

                // レンダリング結果を構築
                let render_result = RenderResult {
                    output_path: self.config.output_path.clone(),
                    duration: entry.metadata.duration,
                    total_frames: (entry.metadata.duration.as_seconds() * self.config.frame_rate)
                        as u64,
                    render_time: std::time::Duration::from_secs(0), // キャッシュからなので0
                    average_render_fps: 0.0,                        // キャッシュからなので0
                    from_cache: true,
                };

                return Ok(render_result);
            }
        }

        // プロジェクトのレンダリングを実行
        // 実際の実装では、ここでレンダリングプロセスを実行

        // Set rendering stage
        self.progress.set_stage(RenderStage::Rendering);

        // Create a compositor
        let mut compositor =
            TrackCompositor::new(self.project.timeline.clone(), self.project.assets.clone());

        // Set progress tracker
        compositor.set_progress_tracker(self.progress.clone());

        // Render using the compositor
        if let Err(e) = compositor.compose(&self.config) {
            return Err(RenderError::Timeline(format!("Composition error: {}", e)));
        }

        // レンダリングが完了したら、結果を返す
        let now = std::time::Instant::now();
        let elapsed = now.elapsed();
        let timeline_duration = Self::calculate_timeline_duration(&self.project);
        let total_frames = (timeline_duration.as_seconds() * self.config.frame_rate) as u64;
        let render_result = RenderResult {
            output_path: self.config.output_path.clone(),
            duration: timeline_duration,
            total_frames,
            render_time: elapsed,
            average_render_fps: total_frames as f64 / elapsed.as_secs_f64(),
            from_cache: false,
        };

        // プロジェクト全体をキャッシュに追加（auto_loading中は行わない）
        if !self.auto_loading && self.config.use_cache && self.cache.is_some() {
            let cache = Arc::get_mut(self.cache.as_mut().unwrap()).unwrap();

            // プロジェクト全体のハッシュを計算（簡略化のためproject_id + 固定値を使用）
            let project_hash = 12345; // 実際の実装では、プロジェクト構造から適切なハッシュを計算

            // すべてのアセットIDを取得（これは実際のアセットIDではなく、ダミー）
            let dummy_asset_id = crate::project::AssetId::new();

            // キャッシュに追加
            let _ = cache.add(
                dummy_asset_id,
                project_hash,
                &self.config.output_path,
                render_result.duration,
            );
        }

        // プロジェクトのレンダリング前にキャンセルされたか確認
        if self.progress.is_cancelled() {
            return Err(RenderError::Cancelled);
        }

        // progress更新
        // 新しいAPIは個別のアップデートをサポートしていないので、ステージのみ設定
        self.progress.set_stage(RenderStage::Complete);

        Ok(render_result)
    }

    /// Renders the project asynchronously.
    ///
    /// # Arguments
    ///
    /// * `callback` - Optional callback function to call when rendering is complete
    ///
    /// # Returns
    ///
    /// A join handle for the rendering thread.
    pub fn render_async<F>(
        mut self,
        callback: Option<F>,
    ) -> thread::JoinHandle<Result<RenderResult, RenderError>>
    where
        F: FnOnce(Result<RenderResult, RenderError>) + Send + 'static,
    {
        thread::spawn(move || {
            let result = self.render();
            if let Some(callback) = callback {
                callback(result.clone());
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
        // 新しいAPIはRenderProgressを直接提供しないので、必要な情報を手動で構築
        let current_stage = self.progress.get_stage();
        let _is_cancelled = self.progress.is_cancelled();

        let now = std::time::Instant::now();
        let elapsed = self
            .start_time
            .map_or(std::time::Duration::from_secs(0), |st| {
                now.duration_since(st)
            });

        let timeline_duration = Self::calculate_timeline_duration(&self.project);
        let total_frames = (timeline_duration.as_seconds() * self.config.frame_rate) as u64;

        // 簡略化されたRenderProgressを返す
        Some(RenderProgress {
            frames_completed: 0, // 正確な値は取得できない
            total_frames,
            current_position: crate::utility::time::TimePosition::from_seconds(0.0),
            total_duration: timeline_duration,
            elapsed,
            estimated_remaining: None, // 正確な値は取得できない
            render_fps: 0.0,           // 正確な値は取得できない
            current_stage,
        })
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

    // コアキャッシュディレクトリを使用してキャッシュを初期化
    if pipeline.config.use_cache {
        // コアモジュールからキャッシュディレクトリを取得できない場合は一時ディレクトリを使用
        let default_cache_dir = std::env::temp_dir().join("edv_cache");
        let _ = pipeline.init_cache(default_cache_dir, None);
    }

    // 設定に応じてアセットを自動読み込み
    if pipeline.config.auto_load_assets {
        let _ = pipeline.auto_load_assets();
    }

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
