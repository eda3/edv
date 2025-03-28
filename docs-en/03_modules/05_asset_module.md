# edv - Asset Module Implementation

This document provides detailed implementation guidelines for the Asset module of the edv application.

## Overview

The Asset module handles media assets and their metadata within the edv application. It is responsible for asset management, metadata extraction, proxy generation, and caching of asset information. This module serves as a bridge between the physical media files and their representation within the edv project.

## Structure

```
src/asset/
├── mod.rs                 // Module exports
├── asset.rs               // Core asset structure
├── manager.rs             // Asset management
├── metadata/              // Metadata handling
│   ├── mod.rs             // Metadata exports
│   ├── extractor.rs       // Metadata extraction
│   ├── parser.rs          // Metadata parsing
│   └── schema.rs          // Metadata schema
├── proxy/                 // Proxy handling
│   ├── mod.rs             // Proxy exports
│   ├── generator.rs       // Proxy generation
│   └── manager.rs         // Proxy management
├── cache/                 // Asset caching
│   ├── mod.rs             // Cache exports
│   ├── cache.rs           // Cache implementation
│   └── policy.rs          // Cache policies
└── utils/                 // Asset-specific utilities
    ├── mod.rs             // Utility exports
    ├── media_info.rs      // Media information
    └── indexer.rs         // Asset indexing
```

## Key Components

### Asset Structure (asset.rs)

The core asset data structure that represents media files:

```rust
/// Unique identifier for an asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub Uuid);

impl AssetId {
    /// Create a new random asset ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Represents a media asset within the system
pub struct Asset {
    /// Unique asset identifier
    pub id: AssetId,
    /// Path to the media file
    pub path: PathBuf,
    /// Extracted metadata
    pub metadata: AssetMetadata,
    /// Path to proxy file if available
    pub proxy_path: Option<PathBuf>,
    /// Asset status information
    pub status: AssetStatus,
    /// User-defined tags
    pub tags: HashMap<String, String>,
}

impl Asset {
    /// Create a new asset from a file path
    pub fn new(path: PathBuf) -> Result<Self> {
        let id = AssetId::new();
        
        // Validate file exists
        if !path.exists() {
            return Err(Error::FileNotFound(path.to_string_lossy().to_string()));
        }
        
        // Create initial asset without metadata
        let asset = Self {
            id,
            path,
            metadata: AssetMetadata::default(),
            proxy_path: None,
            status: AssetStatus::Pending,
            tags: HashMap::new(),
        };
        
        Ok(asset)
    }
    
    /// Check if the asset is a video file
    pub fn is_video(&self) -> bool {
        self.metadata.video_streams.len() > 0
    }
    
    /// Check if the asset is an audio file
    pub fn is_audio(&self) -> bool {
        self.metadata.audio_streams.len() > 0 && self.metadata.video_streams.len() == 0
    }
    
    /// Check if the asset has a proxy
    pub fn has_proxy(&self) -> bool {
        self.proxy_path.is_some() && self.proxy_path.as_ref().unwrap().exists()
    }
    
    /// Get the display name for the asset
    pub fn display_name(&self) -> String {
        self.path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown Asset".to_string())
    }
    
    /// Get the best path for editing operations (proxy if available, original otherwise)
    pub fn editing_path(&self) -> &Path {
        if self.has_proxy() {
            self.proxy_path.as_ref().unwrap()
        } else {
            &self.path
        }
    }
}

/// Asset metadata containing information about the media file
pub struct AssetMetadata {
    /// Duration of the media
    pub duration: Option<Duration>,
    /// Dimensions if video
    pub dimensions: Option<(u32, u32)>,
    /// Frame rate if video
    pub frame_rate: Option<f64>,
    /// Creation date if available
    pub creation_date: Option<DateTime<Utc>>,
    /// Video streams information
    pub video_streams: Vec<VideoStreamInfo>,
    /// Audio streams information
    pub audio_streams: Vec<AudioStreamInfo>,
    /// Subtitle streams information
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    /// Overall bitrate
    pub bitrate: Option<u64>,
    /// File format
    pub format: Option<String>,
    /// Additional metadata as key-value pairs
    pub extra: HashMap<String, String>,
}

impl AssetMetadata {
    /// Create a default empty metadata structure
    pub fn default() -> Self {
        Self {
            duration: None,
            dimensions: None,
            frame_rate: None,
            creation_date: None,
            video_streams: Vec::new(),
            audio_streams: Vec::new(),
            subtitle_streams: Vec::new(),
            bitrate: None,
            format: None,
            extra: HashMap::new(),
        }
    }
}

/// Status of an asset within the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    /// Asset is registered but metadata not yet extracted
    Pending,
    /// Metadata extraction is in progress
    Analyzing,
    /// Asset is ready for use
    Ready,
    /// Asset has errors
    Error,
    /// Asset is being processed (proxy generation etc.)
    Processing,
}
```

### Asset Manager (manager.rs)

The component responsible for managing assets:

```rust
/// Manages assets within the application
pub struct AssetManager {
    /// Collection of assets indexed by ID
    assets: HashMap<AssetId, Asset>,
    /// FFmpeg wrapper for media operations
    ffmpeg: FFmpegWrapper,
    /// Metadata extractor
    metadata_extractor: MetadataExtractor,
    /// Proxy generator
    proxy_generator: ProxyGenerator,
    /// Asset cache
    cache: AssetCache,
    /// Application configuration
    config: AppConfig,
}

impl AssetManager {
    /// Create a new asset manager
    pub fn new(config: AppConfig, ffmpeg: FFmpegWrapper) -> Self {
        Self {
            assets: HashMap::new(),
            ffmpeg,
            metadata_extractor: MetadataExtractor::new(ffmpeg.clone()),
            proxy_generator: ProxyGenerator::new(ffmpeg.clone(), &config),
            cache: AssetCache::new(&config.asset_cache_dir),
            config,
        }
    }
    
    /// Register a new asset from a file path
    pub fn register_asset(&mut self, path: &Path) -> Result<AssetId> {
        // Create new asset
        let mut asset = Asset::new(path.to_path_buf())?;
        
        // Check for cached metadata
        if let Some(cached_metadata) = self.cache.get_metadata(path) {
            asset.metadata = cached_metadata;
            asset.status = AssetStatus::Ready;
        } else {
            // Queue for metadata extraction
            asset.status = AssetStatus::Pending;
        }
        
        let id = asset.id;
        self.assets.insert(id, asset);
        
        Ok(id)
    }
    
    /// Extract metadata for an asset
    pub fn extract_metadata(&mut self, asset_id: AssetId) -> Result<()> {
        let asset = self.assets.get_mut(&asset_id)
            .ok_or(Error::AssetNotFound(asset_id))?;
            
        // Update status
        asset.status = AssetStatus::Analyzing;
        
        // Extract metadata
        let metadata = self.metadata_extractor.extract(&asset.path)?;
        
        // Update asset
        asset.metadata = metadata;
        asset.status = AssetStatus::Ready;
        
        // Cache metadata
        self.cache.store_metadata(&asset.path, &asset.metadata)?;
        
        Ok(())
    }
    
    /// Generate proxy for an asset
    pub fn generate_proxy(&mut self, asset_id: AssetId) -> Result<()> {
        let asset = self.assets.get_mut(&asset_id)
            .ok_or(Error::AssetNotFound(asset_id))?;
            
        // Check if asset has metadata
        if asset.status != AssetStatus::Ready {
            return Err(Error::AssetNotReady(asset_id));
        }
        
        // Update status
        asset.status = AssetStatus::Processing;
        
        // Generate proxy
        let proxy_path = self.proxy_generator.generate(&asset.path, &asset.metadata)?;
        
        // Update asset
        asset.proxy_path = Some(proxy_path);
        asset.status = AssetStatus::Ready;
        
        Ok(())
    }
    
    /// Get an asset by ID
    pub fn get_asset(&self, asset_id: AssetId) -> Option<&Asset> {
        self.assets.get(&asset_id)
    }
    
    /// Get a mutable reference to an asset by ID
    pub fn get_asset_mut(&mut self, asset_id: AssetId) -> Option<&mut Asset> {
        self.assets.get_mut(&asset_id)
    }
    
    /// Remove an asset from the manager
    pub fn remove_asset(&mut self, asset_id: AssetId) -> Result<()> {
        let asset = self.assets.remove(&asset_id)
            .ok_or(Error::AssetNotFound(asset_id))?;
            
        // Remove proxy if exists
        if let Some(proxy_path) = asset.proxy_path {
            if proxy_path.exists() {
                std::fs::remove_file(proxy_path)?;
            }
        }
        
        Ok(())
    }
    
    /// List all managed assets
    pub fn list_assets(&self) -> Vec<&Asset> {
        self.assets.values().collect()
    }
    
    /// Find assets by name pattern
    pub fn find_assets_by_name(&self, pattern: &str) -> Vec<&Asset> {
        let regex = Regex::new(pattern).unwrap_or_else(|_| Regex::new(".*").unwrap());
        
        self.assets.values()
            .filter(|a| {
                let name = a.path.file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                regex.is_match(&name)
            })
            .collect()
    }
}
```

### Metadata Extractor (metadata/extractor.rs)

Extracts metadata from media files:

```rust
/// Extracts metadata from media files
pub struct MetadataExtractor {
    /// FFmpeg wrapper for media operations
    ffmpeg: FFmpegWrapper,
}

impl MetadataExtractor {
    /// Create a new metadata extractor
    pub fn new(ffmpeg: FFmpegWrapper) -> Self {
        Self {
            ffmpeg,
        }
    }
    
    /// Extract metadata from a media file
    pub fn extract(&self, path: &Path) -> Result<AssetMetadata> {
        // Execute FFmpeg command to get media info
        let output = self.ffmpeg.run_info_command(path)?;
        
        // Parse FFmpeg output
        let parser = MetadataParser::new();
        let metadata = parser.parse(&output)?;
        
        Ok(metadata)
    }
    
    /// Extract thumbnail from a media file
    pub fn extract_thumbnail(&self, path: &Path, time: Option<TimePosition>, output_path: &Path) -> Result<()> {
        // Determine time position (default to 10% of duration)
        let time_pos = if let Some(t) = time {
            t
        } else {
            // Get media info to determine duration
            let info = self.ffmpeg.get_media_info(path)?;
            if let Some(duration) = info.duration {
                TimePosition::from_seconds(duration.as_seconds() * 0.1)
            } else {
                TimePosition::from_seconds(0.0)
            }
        };
        
        // Generate thumbnail
        self.ffmpeg.extract_frame(path, time_pos, output_path)
    }
}
```

### Proxy Generator (proxy/generator.rs)

Generates proxy files for more efficient editing:

```rust
/// Generates proxy media files for more efficient editing
pub struct ProxyGenerator {
    /// FFmpeg wrapper for media operations
    ffmpeg: FFmpegWrapper,
    /// Proxy generation configuration
    config: ProxyConfig,
}

impl ProxyGenerator {
    /// Create a new proxy generator
    pub fn new(ffmpeg: FFmpegWrapper, app_config: &AppConfig) -> Self {
        Self {
            ffmpeg,
            config: app_config.proxy.clone(),
        }
    }
    
    /// Generate a proxy file for a media asset
    pub fn generate(&self, source_path: &Path, metadata: &AssetMetadata) -> Result<PathBuf> {
        // Determine target resolution
        let target_resolution = self.determine_target_resolution(metadata);
        
        // Generate unique proxy filename
        let source_filename = source_path.file_name()
            .ok_or_else(|| Error::InvalidPath(source_path.to_string_lossy().to_string()))?;
            
        let proxy_filename = format!(
            "proxy_{}_{}x{}.mp4",
            source_filename.to_string_lossy(),
            target_resolution.0,
            target_resolution.1
        );
        
        let proxy_path = Path::new(&self.config.proxy_dir).join(proxy_filename);
        
        // Create proxy directory if it doesn't exist
        std::fs::create_dir_all(&self.config.proxy_dir)?;
        
        // Build FFmpeg command for proxy generation
        let mut command = FFmpegCommand::new();
        
        command.input(source_path)
            .output(&proxy_path)
            .video_codec("libx264")
            .audio_codec("aac")
            .video_bitrate(self.config.video_bitrate)
            .audio_bitrate(self.config.audio_bitrate)
            .resolution(target_resolution.0, target_resolution.1)
            .preset("fast")
            .additional_arg("-crf", "23")
            .additional_arg("-g", "60");
            
        // Execute command
        self.ffmpeg.run_command(command, None)?;
        
        // Verify proxy was created
        if !proxy_path.exists() {
            return Err(Error::ProxyGenerationFailed);
        }
        
        Ok(proxy_path)
    }
    
    /// Determine the target resolution for a proxy based on the original media
    fn determine_target_resolution(&self, metadata: &AssetMetadata) -> (u32, u32) {
        // Get original dimensions
        let original_dims = metadata.dimensions.unwrap_or((1920, 1080));
        
        // Calculate aspect ratio
        let aspect_ratio = original_dims.0 as f64 / original_dims.1 as f64;
        
        // Determine target height based on proxy quality setting
        let target_height = match self.config.quality {
            ProxyQuality::Low => 360,
            ProxyQuality::Medium => 540,
            ProxyQuality::High => 720,
        };
        
        // Calculate width maintaining aspect ratio
        let target_width = (target_height as f64 * aspect_ratio).round() as u32;
        
        // Ensure width is even (required by some codecs)
        let target_width = if target_width % 2 == 1 { target_width + 1 } else { target_width };
        
        (target_width, target_height)
    }
}
```

### Asset Cache (cache/cache.rs)

Caches asset information for performance:

```rust
/// Caches asset information for improved performance
pub struct AssetCache {
    /// Directory where cache is stored
    cache_dir: PathBuf,
    /// Cache entries
    entries: HashMap<PathBuf, CacheEntry>,
    /// Cache policy
    policy: CachePolicy,
}

impl AssetCache {
    /// Create a new asset cache
    pub fn new(cache_dir: &Path) -> Self {
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(cache_dir).unwrap_or_default();
        
        Self {
            cache_dir: cache_dir.to_path_buf(),
            entries: HashMap::new(),
            policy: CachePolicy::default(),
        }
    }
    
    /// Initialize the cache by loading existing entries
    pub fn initialize(&mut self) -> Result<()> {
        // Scan cache directory
        let entries = std::fs::read_dir(&self.cache_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Only process JSON files
            if path.extension().map_or(false, |ext| ext == "json") {
                // Load cache entry
                if let Ok(cache_entry) = self.load_entry(&path) {
                    // Get the original file path from entry
                    if let Some(original_path) = &cache_entry.file_path {
                        self.entries.insert(original_path.clone(), cache_entry);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get cached metadata for a file if available
    pub fn get_metadata(&self, file_path: &Path) -> Option<AssetMetadata> {
        self.entries.get(file_path).and_then(|entry| {
            // Check if entry is still valid according to policy
            if self.policy.is_valid(entry) {
                entry.metadata.clone()
            } else {
                None
            }
        })
    }
    
    /// Store metadata in cache
    pub fn store_metadata(&mut self, file_path: &Path, metadata: &AssetMetadata) -> Result<()> {
        // Get file modification time
        let modified = std::fs::metadata(file_path)?.modified()?;
        
        // Create cache entry
        let entry = CacheEntry {
            file_path: Some(file_path.to_path_buf()),
            metadata: Some(metadata.clone()),
            created: SystemTime::now(),
            last_accessed: SystemTime::now(),
            file_modified: modified,
        };
        
        // Store in memory cache
        self.entries.insert(file_path.to_path_buf(), entry.clone());
        
        // Store on disk
        self.save_entry(file_path, &entry)?;
        
        Ok(())
    }
    
    /// Clear outdated cache entries
    pub fn cleanup(&mut self) -> Result<usize> {
        let mut removed_count = 0;
        
        // Create list of entries to remove
        let to_remove: Vec<PathBuf> = self.entries.iter()
            .filter(|(_, entry)| !self.policy.is_valid(entry))
            .map(|(path, _)| path.clone())
            .collect();
            
        // Remove from memory and disk
        for path in to_remove {
            if let Some(entry) = self.entries.remove(&path) {
                // Remove from disk if we have a file path
                if let Some(original_path) = &entry.file_path {
                    let cache_path = self.get_cache_path(original_path);
                    if cache_path.exists() {
                        std::fs::remove_file(cache_path)?;
                        removed_count += 1;
                    }
                }
            }
        }
        
        Ok(removed_count)
    }
    
    /// Load a cache entry from disk
    fn load_entry(&self, cache_file: &Path) -> Result<CacheEntry> {
        // Read file
        let file = std::fs::File::open(cache_file)?;
        let reader = std::io::BufReader::new(file);
        
        // Deserialize JSON
        let entry: CacheEntry = serde_json::from_reader(reader)?;
        
        Ok(entry)
    }
    
    /// Save a cache entry to disk
    fn save_entry(&self, file_path: &Path, entry: &CacheEntry) -> Result<()> {
        // Generate cache file path
        let cache_path = self.get_cache_path(file_path);
        
        // Serialize to JSON
        let file = std::fs::File::create(cache_path)?;
        let writer = std::io::BufWriter::new(file);
        
        serde_json::to_writer_pretty(writer, entry)?;
        
        Ok(())
    }
    
    /// Generate a cache file path for a given file path
    fn get_cache_path(&self, file_path: &Path) -> PathBuf {
        // Hash the file path to create a unique filename
        let mut hasher = DefaultHasher::new();
        file_path.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Create cache filename
        let cache_filename = format!("cache_{}.json", hash);
        
        self.cache_dir.join(cache_filename)
    }
}
```

## Key Interfaces

### Asset Management Interface

The Asset module exposes the following key interfaces for asset management:

- **Asset Registration**: Register media files as assets
- **Asset Metadata**: Extract and manage metadata for assets
- **Asset Retrieval**: Find and retrieve assets by ID or criteria
- **Asset Modification**: Update asset properties and metadata

### Metadata Interface

The metadata system provides interfaces for:

- **Metadata Extraction**: Extract metadata from media files
- **Metadata Parsing**: Parse FFmpeg output into structured metadata
- **Metadata Schema**: Define structure for different types of metadata

### Proxy Management Interface

The proxy system provides interfaces for:

- **Proxy Generation**: Create lower-resolution proxy files
- **Proxy Configuration**: Configure proxy quality and settings
- **Proxy Utilization**: Use proxies for editing operations

### Cache Interface

The cache system provides interfaces for:

- **Cache Management**: Store and retrieve cached information
- **Cache Invalidation**: Determine when cached data is no longer valid
- **Cache Cleanup**: Remove outdated or unnecessary cache entries

## Performance Considerations

- **Lazy Metadata Extraction**: Extract detailed metadata only when needed
- **Efficient Proxy Handling**: Generate proxies asynchronously and in background
- **Caching Strategy**: Cache frequently accessed metadata for performance
- **Memory Management**: Balance in-memory cache with disk storage

## Future Enhancements

- **Advanced Media Analysis**: Extract deeper insights about media content
- **Cloud Storage Support**: Support for assets stored in cloud services
- **Distributed Asset Management**: Coordination of assets across multiple systems
- **Metadata Enrichment**: Integration with external metadata sources
- **Smart Proxy Optimization**: Adaptive proxy quality based on device capabilities

This modular Asset implementation provides a foundation for efficient and scalable media asset management, supporting the needs of a command-line video editing tool while maintaining performance and flexibility. 