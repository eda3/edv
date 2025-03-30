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
            progress: SharedProgressTracker::new(total_frames, timeline_duration),
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
    pub fn set_progress_callback(&self, callback: ProgressCallback) {
        self.progress.set_callback(callback);
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
    pub fn auto_load_assets(&mut self) -> Result<(), RenderError> {
        if !self.config.auto_load_assets {
            return Ok(());
        }

        // マークauto_loadingをtrueに設定してレンダリングをキャッシュに保存
        self.auto_loading = true;
        self.progress.set_stage(RenderStage::PreparingAssets);

        // プロジェクト内のすべてのアセットをロード
        for asset in &self.project.assets {
            // アセットの種類をチェックしてキャッシュが必要なものだけ処理
            if asset.metadata.asset_type == "video" || asset.metadata.asset_type == "audio" {
                let asset_id = asset.id;

                // キャッシュのパラメータハッシュを作成
                let params_hash = if let Some(cache) = &self.cache {
                    // レンダリング設定のハッシュを計算
                    cache.hash_params(&self.config)
                } else {
                    // キャッシュが無効な場合はダミーハッシュ
                    0
                };

                // キャッシュが有効で、すでにキャッシュされているかチェック
                if self.config.use_cache && self.cache.is_some() {
                    let cache = self.cache.as_ref().unwrap();
                    if cache.get(asset_id, params_hash).is_some() {
                        // すでにキャッシュされているのでスキップ
                        continue;
                    }
                }

                // アセットが動画なら、最適なサイズとエンコード設定でレンダリング
                if asset.metadata.asset_type == "video" {
                    // シンプルな設定でアセットをレンダリング
                    // 実際の実装では、ここでアセットを適切なサイズとエンコード設定でレンダリング

                    // progress更新
                    self.progress
                        .update(0, crate::utility::time::TimePosition::from_seconds(0.0));

                    // アセットパスから一時ファイルパスを生成
                    let temp_file = std::env::temp_dir().join(format!("autoload_{}.mp4", asset_id));

                    // FFmpegを使用してアセットを変換（この例では実装を省略）
                    // 実際の実装では、ここでFFmpegでアセットを変換

                    // キャッシュに追加
                    if self.config.use_cache && self.cache.is_some() {
                        let cache = Arc::get_mut(self.cache.as_mut().unwrap()).unwrap();
                        let duration = asset
                            .metadata
                            .duration
                            .unwrap_or_else(|| Duration::from_seconds(0.0));

                        if temp_file.exists() {
                            let _ = cache.add(asset_id, params_hash, &temp_file, duration);
                            // 一時ファイルを削除 (オプション)
                            let _ = std::fs::remove_file(&temp_file);
                        }
                    }
                }
            }
        }

        self.auto_loading = false;
        self.progress.set_stage(RenderStage::Ready);

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
                // キャッシュから結果を作成
                let progress = self.progress.get_progress().ok_or_else(|| {
                    RenderError::Timeline("Failed to get progress information".to_string())
                })?;

                // キャッシュファイルを出力にコピー
                if std::fs::copy(&entry.path, &self.config.output_path).is_ok() {
                    // キャッシュから結果を返す
                    let render_result = RenderResult {
                        output_path: self.config.output_path.clone(),
                        duration: entry.metadata.duration,
                        total_frames: progress.total_frames,
                        render_time: std::time::Duration::from_secs(0), // キャッシュからなので0
                        average_render_fps: 0.0,                        // キャッシュからなので0
                        from_cache: true,
                    };

                    return Ok(render_result);
                }
            }
        }

        // Create compositor
        let mut compositor =
            TrackCompositor::new(self.project.timeline.clone(), self.project.assets.clone());

        // Set progress tracker
        compositor.set_progress_tracker(self.progress.clone());

        // Enable complex timeline optimization if configured
        compositor.set_optimize_complex(self.config.optimize_complex_timelines);

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
