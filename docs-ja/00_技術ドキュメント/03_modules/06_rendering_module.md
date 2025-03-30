# Rendering Module

## Overview

The Rendering Module provides functionality to render EDV projects into final video files. This module implements features for compositing timelines with multiple tracks, applying effects, monitoring progress, and managing render caches.

## Structure

The Rendering Module is located in the `src/project/rendering` directory and consists of the following files:

- `mod.rs`: Module entry point and error type definitions
- `config.rs`: Rendering configuration definitions
- `pipeline.rs`: Rendering pipeline implementation
- `compositor.rs`: Track composition functionality
- `cache.rs`: Rendering cache management
- `progress.rs`: Rendering progress tracking

## Key Components

### RenderConfig

The `RenderConfig` struct manages rendering settings, including output format, resolution, frame rate, codecs, and various options.

```rust
pub struct RenderConfig {
    /// Output file path
    pub output_path: PathBuf,
    /// Video resolution width (in pixels)
    pub width: u32,
    /// Video resolution height (in pixels)
    pub height: u32,
    /// Video frame rate (frames per second)
    pub frame_rate: f64,
    /// Video codec to use
    pub video_codec: VideoCodec,
    /// Video quality (1-100, higher is better)
    pub video_quality: u32,
    /// Audio codec to use
    pub audio_codec: AudioCodec,
    /// Audio quality (1-100, higher is better)
    pub audio_quality: u32,
    /// Output container format
    pub format: OutputFormat,
    // Other settings...
}
```

This class implements the builder pattern, allowing settings to be constructed using method chaining:

```rust
let config = RenderConfig::new(PathBuf::from("output.mp4"))
    .with_resolution(1920, 1080)
    .with_frame_rate(30.0)
    .with_video_settings(VideoCodec::H264, 80)
    .with_audio_settings(AudioCodec::AAC, 80)
    .with_format(OutputFormat::MP4);
```

### RenderPipeline

`RenderPipeline` is the central component of the rendering process, managing the entire process of rendering a project's timeline to a video file.

```rust
pub struct RenderPipeline {
    /// The project being rendered
    project: Project,
    /// Rendering configuration
    config: RenderConfig,
    /// Progress tracker for the rendering process
    progress: SharedProgressTracker,
    /// Start time of the rendering process
    start_time: Option<std::time::Instant>,
    /// Cache for rendered assets
    cache: Option<Arc<RenderCache>>,
    /// Whether the pipeline is currently in auto-loading mode
    auto_loading: bool,
}
```

Key features:

1. Initializing the rendering pipeline based on project and configuration
2. Cache initialization and management
3. Support for asynchronous rendering
4. Cancellation of rendering processes
5. Monitoring and reporting progress

### TrackCompositor

`TrackCompositor` composes multiple tracks to generate the final video frames, placing clips temporally and applying effects.

```rust
pub struct TrackCompositor {
    // Fields related to track composition
    // ...
}
```

Key features:

1. Composing tracks of different media types
2. Temporal alignment of clips
3. Applying effects
4. Rendering frames

### RenderCache

`RenderCache` enhances performance by caching rendered assets for reuse.

```rust
pub struct RenderCache {
    // Fields related to caching
    // ...
}
```

Key features:

1. Storing and retrieving cache entries
2. Managing cache size
3. Invalidating and updating cache

## Usage Examples

### Basic Rendering

```rust
// Prepare project and configuration
let project = Project::load("my_project.edv")?;
let config = RenderConfig::new(PathBuf::from("output.mp4"))
    .with_resolution(1920, 1080)
    .with_frame_rate(30.0);

// Use the simple rendering function
let result = render_project_simple(project, &PathBuf::from("output.mp4"))?;
println!("Rendering complete: {:?}", result.output_path);
```

### Detailed Rendering Configuration

```rust
// Prepare project and detailed configuration
let project = Project::load("my_project.edv")?;
let config = RenderConfig::new(PathBuf::from("output.mp4"))
    .with_resolution(1920, 1080)
    .with_frame_rate(30.0)
    .with_video_settings(VideoCodec::H265, 85)
    .with_audio_settings(AudioCodec::AAC, 80)
    .with_format(OutputFormat::MP4)
    .with_cache(true)
    .with_threads(8);

// Create a rendering pipeline
let mut pipeline = RenderPipeline::new(project, config);

// Initialize the cache
pipeline.init_cache(PathBuf::from("./cache"), Some(10 * 1024 * 1024 * 1024))?;

// Set progress callback
pipeline.set_progress_callback(|progress| {
    println!("Progress: {}%", progress.percent_complete());
    false // Don't cancel
});

// Execute rendering
let result = pipeline.render()?;
println!("Rendering complete: {:?}", result.output_path);
```

### Asynchronous Rendering

```rust
// Prepare project and configuration
let project = Project::load("my_project.edv")?;
let config = RenderConfig::new(PathBuf::from("output.mp4"));

// Create a rendering pipeline
let pipeline = RenderPipeline::new(project, config);

// Execute rendering asynchronously
let handle = pipeline.render_async(Some(|result| {
    match result {
        Ok(r) => println!("Rendering complete: {:?}", r.output_path),
        Err(e) => println!("Rendering error: {:?}", e),
    }
}));

// Perform other operations...

// Wait for rendering result if needed
let result = handle.join().unwrap();
```

## Design Considerations

1. **Performance**: Rendering complex timelines has a high computational cost, so optimizing performance is crucial. Caching and parallel processing implementation help with this.

2. **Error Handling**: Various operations such as FFmpeg operations, file I/O, and composition processing can produce errors. The module implements comprehensive error handling to properly handle these errors.

3. **Extensibility**: The design adopts a highly extensible approach to allow for the future addition of new codecs, formats, and effects.

4. **Resource Management**: The rendering process consumes significant memory and CPU resources, so efficient resource management is essential.

## Implementation Status Update (2024)

Phase 3 of the EDV project, which includes enhancements to the rendering module, officially started on April 1, 2024. The current implementation status is at approximately 15% completion.

Key progress:
- Core rendering infrastructure is in place with the module structure established
- Basic support for multiple video and audio tracks has been implemented
- The foundation for advanced effects and filters has been completed
- Integration with FFmpeg for output generation is working well

Upcoming tasks:
- Implementing keyframe support for effects and transitions
- Adding track locking and visibility controls
- Enhancing the cache system for better performance
- Adding compositor features for blending modes and masks

The expected completion date for all rendering module enhancements is May 24, 2024, as part of the overall Phase 3 deliverables.

## Limitations and Future Improvements

1. **GPU Acceleration**: The current version only supports CPU-based rendering. GPU acceleration will be added in future versions.

2. **Hardware Encoding**: Support for hardware encoding is limited. Future releases will improve support for NVENC, QuickSync, and AMD encoders.

3. **Advanced Effects**: The current version only supports basic effects. More advanced effects and transitions will be added in future versions.

4. **Distributed Rendering**: Distribution of rendering jobs across multiple machines is not currently supported but is being considered as a future feature. 