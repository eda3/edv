# edv - Module Implementation Details

This document provides detailed implementation guidelines for each module of the edv application. It covers the structure, interfaces, and important implementation considerations for each module based on the design specifications.

## 1. CLI Module

### Structure
```
src/cli/
├── mod.rs
├── app.rs
├── commands/
│   ├── mod.rs
│   ├── trim.rs
│   ├── concat.rs
│   ├── filter.rs
│   └── ...
└── progress.rs
```

### Key Components

#### App (app.rs)
```rust
pub struct App {
    config: AppConfig,
    command_registry: CommandRegistry,
}

impl App {
    pub fn new() -> Result<Self>;
    pub fn run() -> Result<()>;
    pub fn register_commands(&mut self);
    pub fn parse_args(&self) -> ArgMatches;
    pub fn execute_command(&self, matches: ArgMatches) -> Result<()>;
}
```

#### Command Trait (commands/mod.rs)
```rust
pub trait Command {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn usage(&self) -> String;
    fn configure_args(&self, app: Command) -> Command;
    fn execute(&self, context: ExecutionContext, args: ArgMatches) -> Result<()>;
}
```

#### Command Implementations
Each command will implement the Command trait and handle specific operations:
- `TrimCommand`: Handle video trimming operations
- `ConcatCommand`: Handle video concatenation
- `FilterCommand`: Apply various filters to videos
- `AudioCommand`: Process audio operations
- `ConvertCommand`: Handle format conversion
- `SubtitleCommand`: Process subtitle operations
- `ProjectCommand`: Manage project operations
- `BatchCommand`: Process batch operations

#### Progress Display (progress.rs)
```rust
pub struct ProgressDisplay {
    progress_bar: ProgressBar,
    format: ProgressFormat,
    total_steps: u64,
}

impl ProgressDisplay {
    pub fn new(total_steps: u64, format: ProgressFormat) -> Self;
    pub fn update(&self, progress: u64, message: Option<String>);
    pub fn finish(&self, message: String);
    pub fn error(&self, message: String);
}
```

### Implementation Notes
- Use clap for argument parsing and help generation
- Implement a plugin-like command registry for easy extension
- Create a consistent terminal UI with colored output and progress bars
- Handle keyboard interrupts gracefully
- Ensure all commands have consistent help text format

## 2. Core Module

### Structure
```
src/core/
├── mod.rs
├── config.rs
├── error.rs
└── context.rs
```

### Key Components

#### Configuration (config.rs)
```rust
pub struct AppConfig {
    ffmpeg: FfmpegConfig,
    processing: ProcessingConfig,
    temp_dir: PathBuf,
    default_output_format: String,
}

impl AppConfig {
    pub fn load_default() -> Result<Self>;
    pub fn load_from_file(path: &Path) -> Result<Self>;
    pub fn save_to_file(&self, path: &Path) -> Result<()>;
    pub fn merge_with_env(&mut self) -> Result<()>;
    pub fn get_ffmpeg_config(&self) -> &FfmpegConfig;
    pub fn get_processing_config(&self) -> &ProcessingConfig;
}
```

#### Error Handling (error.rs)
```rust
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Input error: {0}")]
    Input(String),
    
    #[error("Output error: {0}")]
    Output(String),
    
    #[error("Parameter error: {0}")]
    Parameter(String),
    
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    
    #[error("Processing error: {0}")]
    Processing(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Project error: {0}")]
    Project(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

#### Execution Context (context.rs)
```rust
pub struct ExecutionContext {
    config: AppConfig,
    pipeline: Arc<ProcessingPipeline>,
    temp_dir: TempDir,
}

impl ExecutionContext {
    pub fn new(config: AppConfig) -> Result<Self>;
    pub fn get_config(&self) -> &AppConfig;
    pub fn get_pipeline(&self) -> &Arc<ProcessingPipeline>;
    pub fn get_temp_path(&self, filename: &str) -> PathBuf;
    pub fn cleanup(&self) -> Result<()>;
}
```

### Implementation Notes
- Use serde for configuration serialization/deserialization
- Implement a robust error type system with helpful error messages
- Create a centralized execution context for sharing state
- Use temporary directories that clean up automatically
- Implement environment variable overrides for all configurations

## 3. Processing Module

### Structure
```
src/processing/
├── mod.rs
├── pipeline.rs
├── scheduler.rs
├── operations/
│   ├── mod.rs
│   ├── trim.rs
│   ├── concat.rs
│   ├── filter.rs
│   └── ...
└── ffmpeg.rs
```

### Key Components

#### Processing Pipeline (pipeline.rs)
```rust
pub struct ProcessingPipeline {
    config: AppConfig,
    ffmpeg: FfmpegWrapper,
}

impl ProcessingPipeline {
    pub fn new(config: AppConfig) -> Result<Self>;
    pub fn execute(&self, operation: Box<dyn Operation>, progress: ProgressBar) -> Result<()>;
    pub fn get_media_info(&self, path: &Path) -> Result<MediaInfo>;
}
```

#### Operation Trait (operations/mod.rs)
```rust
pub trait Operation {
    fn validate(&self) -> Result<()>;
    fn create_execution_plan(&self) -> Result<ExecutionPlan>;
    fn post_process(&self) -> Result<()>;
}
```

#### Execution Plan (operations/mod.rs)
```rust
pub struct ExecutionPlan {
    steps: Vec<ExecutionStep>,
}

pub struct ExecutionStep {
    description: String,
    command_type: CommandType,
}

pub enum CommandType {
    FFmpeg(FfmpegCommand),
    Custom(Box<dyn Fn(&AppConfig, ProgressBar) -> Result<()>>),
}
```

#### FFmpeg Wrapper (ffmpeg.rs)
```rust
pub struct FfmpegWrapper {
    binary_path: PathBuf,
    config: FfmpegConfig,
}

impl FfmpegWrapper {
    pub fn new(app_config: &AppConfig) -> Result<Self>;
    pub fn run_command(&self, command: FfmpegCommand, progress: Option<ProgressBar>) -> Result<()>;
    pub fn get_media_info(&self, path: &Path) -> Result<MediaInfo>;
    pub fn detect_ffmpeg() -> Result<PathBuf>;
}

pub struct FfmpegCommand {
    args: Vec<String>,
}

impl FfmpegCommand {
    pub fn new() -> Self;
    pub fn add_input(&mut self, input: &Path) -> &mut Self;
    pub fn add_output(&mut self, output: &Path) -> &mut Self;
    pub fn add_option(&mut self, option: &str) -> &mut Self;
    pub fn add_filter(&mut self, filter: &str) -> &mut Self;
    pub fn build(&self) -> Vec<String>;
}
```

#### Task Scheduler (scheduler.rs)
```rust
pub struct TaskScheduler {
    max_parallel_tasks: usize,
    pipeline: Arc<ProcessingPipeline>,
}

impl TaskScheduler {
    pub fn new(max_parallel_tasks: usize, pipeline: Arc<ProcessingPipeline>) -> Self;
    pub fn schedule(&self, tasks: Vec<Box<dyn Operation>>, progress: MultiProgress) -> Result<Vec<Result<()>>>;
    pub fn optimize_execution_order(&self, tasks: &mut Vec<Box<dyn Operation>>) -> Result<()>;
}
```

### Implementation Notes
- Ensure the FFmpeg wrapper handles different FFmpeg versions
- Implement robust progress monitoring for FFmpeg operations
- Create a flexible operation API for adding new video operations
- Use parallelism for batch operations where possible
- Implement graceful error handling and cleanup of temporary files

## 4. Project Module

### Structure
```
src/project/
├── mod.rs
├── state.rs
├── timeline.rs
├── history.rs
└── serialization.rs
```

### Key Components

#### Project State (state.rs)
```rust
pub struct ProjectState {
    timeline: Timeline,
    assets: AssetCollection,
    history: EditHistory,
    metadata: ProjectMetadata,
    dirty: bool,
}

impl ProjectState {
    pub fn new() -> Self;
    pub fn load_from_file(path: &Path) -> Result<Self>;
    pub fn save_to_file(&self, path: &Path) -> Result<()>;
    pub fn apply_edit(&mut self, edit: Edit) -> Result<()>;
    pub fn undo(&mut self) -> Result<()>;
    pub fn redo(&mut self) -> Result<()>;
    pub fn is_dirty(&self) -> bool;
    pub fn mark_clean(&mut self);
}
```

#### Timeline (timeline.rs)
```rust
pub struct Timeline {
    tracks: HashMap<TrackId, Track>,
    next_track_id: u32,
    next_clip_id: u32,
    duration: Duration,
    current_position: TimePosition,
}

impl Timeline {
    pub fn new() -> Self;
    pub fn add_track(&mut self, track_type: TrackType) -> Result<TrackId>;
    pub fn remove_track(&mut self, track_id: TrackId) -> Result<()>;
    pub fn add_clip(&mut self, track_id: TrackId, clip: Clip) -> Result<ClipId>;
    pub fn remove_clip(&mut self, track_id: TrackId, clip_id: ClipId) -> Result<()>;
    pub fn move_clip(&mut self, track_id: TrackId, clip_id: ClipId, position: TimePosition) -> Result<()>;
    pub fn resize_clip(&mut self, track_id: TrackId, clip_id: ClipId, new_duration: Duration) -> Result<()>;
    pub fn get_duration(&self) -> Duration;
    pub fn set_current_position(&mut self, position: TimePosition) -> Result<()>;
}
```

#### Edit History (history.rs)
```rust
pub struct EditHistory {
    history: Vec<ProjectSnapshot>,
    current_position: usize,
    max_history_size: usize,
}

impl EditHistory {
    pub fn new(max_history_size: usize) -> Self;
    pub fn push(&mut self, snapshot: ProjectSnapshot);
    pub fn undo(&mut self) -> Option<ProjectSnapshot>;
    pub fn redo(&mut self) -> Option<ProjectSnapshot>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn clear(&mut self);
}
```

### Implementation Notes
- Use serde for serialization of project files
- Implement a non-destructive editing system
- Create an efficient history management system with limited memory usage
- Ensure thread safety for project state
- Implement auto-save functionality for crash recovery

## 5. Asset Module

### Structure
```
src/asset/
├── mod.rs
├── manager.rs
├── metadata.rs
└── proxy.rs
```

### Key Components

#### Asset Manager (manager.rs)
```rust
pub struct AssetManager {
    assets: HashMap<AssetId, Asset>,
    metadata_cache: MetadataCache,
    proxy_settings: ProxySettings,
}

impl AssetManager {
    pub fn new(proxy_settings: ProxySettings) -> Self;
    pub fn import_asset(&mut self, path: &Path) -> Result<AssetId>;
    pub fn get_asset(&self, id: AssetId) -> Option<&Asset>;
    pub fn remove_asset(&mut self, id: AssetId) -> Result<()>;
    pub fn generate_proxy(&mut self, id: AssetId) -> Result<()>;
    pub fn reload_metadata(&mut self, id: AssetId) -> Result<()>;
    pub fn list_assets(&self) -> Vec<&Asset>;
}
```

#### Metadata Extraction (metadata.rs)
```rust
pub struct MetadataCache {
    cache: HashMap<PathBuf, AssetMetadata>,
}

impl MetadataCache {
    pub fn new() -> Self;
    pub fn get(&self, path: &Path) -> Option<&AssetMetadata>;
    pub fn extract(&mut self, path: &Path, ffmpeg: &FfmpegWrapper) -> Result<AssetMetadata>;
    pub fn clear(&mut self);
    pub fn remove(&mut self, path: &Path);
}

pub struct AssetMetadata {
    pub duration: Option<Duration>,
    pub dimensions: Option<(u32, u32)>,
    pub codec: Option<String>,
    pub bitrate: Option<u64>,
    pub frame_rate: Option<f64>,
}
```

#### Proxy Generation (proxy.rs)
```rust
pub struct ProxySettings {
    enabled: bool,
    resolution: ProxyResolution,
    codec: String,
    quality: u8,
    storage_path: PathBuf,
}

impl ProxySettings {
    pub fn new(storage_path: PathBuf) -> Self;
    pub fn is_enabled(&self) -> bool;
    pub fn get_resolution(&self) -> ProxyResolution;
    pub fn get_codec(&self) -> &str;
    pub fn get_quality(&self) -> u8;
    pub fn get_storage_path(&self) -> &Path;
}

pub fn generate_proxy(
    asset_path: &Path,
    output_path: &Path,
    settings: &ProxySettings,
    ffmpeg: &FfmpegWrapper,
) -> Result<()>;
```

### Implementation Notes
- Implement efficient caching of metadata
- Create low-resolution proxies for faster timeline editing
- Use file hashing to detect changes and avoid unnecessary re-processing
- Implement background processing for proxy generation
- Support various asset types (video, audio, image)

## 6. Utility Module

### Structure
```
src/util/
├── mod.rs
├── time.rs
├── fs.rs
└── format.rs
```

### Key Components

#### Time Utilities (time.rs)
```rust
pub struct TimePosition(u64); // nanoseconds

impl TimePosition {
    pub fn from_seconds(seconds: f64) -> Self;
    pub fn from_timecode(timecode: &str) -> Result<Self>;
    pub fn to_seconds(&self) -> f64;
    pub fn to_timecode(&self) -> String;
    pub fn to_ffmpeg_time(&self) -> String;
}

pub struct Duration(u64); // nanoseconds

impl Duration {
    pub fn from_seconds(seconds: f64) -> Self;
    pub fn from_timecode(timecode: &str) -> Result<Self>;
    pub fn to_seconds(&self) -> f64;
    pub fn to_timecode(&self) -> String;
    pub fn to_ffmpeg_time(&self) -> String;
}
```

#### Filesystem Utilities (fs.rs)
```rust
pub fn ensure_directory_exists(path: &Path) -> Result<()>;
pub fn get_unique_filename(dir: &Path, base_name: &str, extension: &str) -> PathBuf;
pub fn calculate_file_hash(path: &Path) -> Result<String>;
pub fn copy_with_progress(source: &Path, target: &Path, progress: Option<&ProgressBar>) -> Result<()>;
pub fn get_file_size(path: &Path) -> Result<u64>;
```

#### Format Utilities (format.rs)
```rust
pub enum VideoContainer {
    MP4,
    MKV,
    WebM,
    AVI,
    MOV,
    GIF,
    Other(String),
}

impl VideoContainer {
    pub fn from_extension(ext: &str) -> Self;
    pub fn to_extension(&self) -> &str;
    pub fn supports_codec(&self, codec: &VideoCodec) -> bool;
    pub fn get_ffmpeg_format(&self) -> &str;
}

pub enum VideoCodec {
    H264,
    H265,
    VP8,
    VP9,
    AV1,
    ProRes,
    Other(String),
}

pub enum AudioCodec {
    AAC,
    MP3,
    FLAC,
    Opus,
    Vorbis,
    WAV,
    Other(String),
}
```

### Implementation Notes
- Create robust time handling for accurate video editing
- Implement filesystem operations with proper error handling
- Create format detection and validation utilities
- Implement progress reporting for file operations
- Create helpers for FFmpeg-specific formatting

## 7. Test Structure

### Unit Tests
Each module will include unit tests in the same file or in a tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function_name() {
        // Test code
    }
}
```

### Integration Tests
```
tests/
├── cli_tests.rs
├── processing_tests.rs
├── project_tests.rs
├── asset_tests.rs
└── common/
    ├── mod.rs
    └── test_utils.rs
```

### Benchmarks
```
benches/
├── processing_bench.rs
├── timeline_bench.rs
└── common/
    ├── mod.rs
    └── bench_utils.rs
```

### Implementation Notes
- Use test fixtures for reproducible tests
- Create mock implementations for external dependencies
- Test edge cases and error handling
- Use property-based testing where appropriate
- Ensure cross-platform test compatibility

## 8. Documentation Structure

### Code Documentation
- Every public function, struct, and trait will have comprehensive documentation
- Examples will be included for common operations
- Documentation will be formatted for rustdoc

### User Documentation
```
docs/
├── user_guide.md
├── installation.md
├── commands/
│   ├── trim.md
│   ├── concat.md
│   └── ...
└── advanced/
    ├── filters.md
    ├── timeline.md
    └── batch_processing.md
```

### Developer Documentation
```
docs/
├── contributing.md
├── architecture.md
├── modules/
│   ├── cli.md
│   ├── processing.md
│   └── ...
└── api/
    ├── operations.md
    ├── plugins.md
    └── ...
```

### Implementation Notes
- Generate API documentation with cargo doc
- Include examples for all major features
- Create comprehensive user guides
- Ensure documentation is accessible and searchable
- Update documentation with each release

This detailed module implementation plan provides guidance for developers to implement each component of the edv application with consistency and clarity. The plan emphasizes clean interfaces, strong type safety, and robust error handling, which are essential for a reliable video editing tool. 