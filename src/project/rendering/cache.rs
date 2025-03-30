/// Rendering cache system.
///
/// This module implements a caching system for rendered assets and intermediate files,
/// improving performance by avoiding redundant rendering operations.
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::project::AssetId;
use crate::project::rendering::{RenderError, Result};
use crate::utility::time::Duration;

/// Hash key for cache entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// Asset ID for the source asset.
    asset_id: AssetId,
    /// Hash of the rendering parameters.
    params_hash: u64,
}

/// Metadata for a cached asset.
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    /// When the cached file was created.
    pub created_at: SystemTime,
    /// Source asset ID.
    pub source_asset_id: AssetId,
    /// Duration of the cached content.
    pub duration: Duration,
    /// Rendering parameters hash.
    pub params_hash: u64,
    /// Size of the cached file in bytes.
    pub file_size: u64,
}

/// Cache entry for a rendered asset.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Path to the cached file.
    pub path: PathBuf,
    /// Metadata for the cached file.
    pub metadata: CacheMetadata,
}

/// Manages the rendering cache.
#[derive(Debug)]
pub struct RenderCache {
    /// Root directory for cache files.
    cache_dir: PathBuf,
    /// Cache entries, indexed by cache key.
    entries: HashMap<CacheKey, CacheEntry>,
    /// Maximum size of the cache in bytes.
    max_size: Option<u64>,
    /// Current size of the cache in bytes.
    current_size: u64,
    /// Whether the cache is enabled.
    enabled: bool,
}

impl RenderCache {
    /// Creates a new render cache with the specified cache directory.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - The directory where cache files will be stored
    /// * `max_size` - Optional maximum size of the cache in bytes
    ///
    /// # Returns
    ///
    /// A new `RenderCache` instance, or an error if the cache directory couldn't be created.
    pub fn new(cache_dir: PathBuf, max_size: Option<u64>) -> Result<Self> {
        // 確認：キャッシュディレクトリが存在しなければ作成
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| RenderError::Io(format!("Failed to create cache directory: {}", e)))?;
        }

        // 新しいキャッシュインスタンスを作成
        let mut cache = Self {
            cache_dir,
            entries: HashMap::new(),
            max_size,
            current_size: 0,
            enabled: true,
        };

        // 既存のキャッシュファイルを読み込み
        cache.load_existing_entries()?;

        Ok(cache)
    }

    /// Loads existing cache entries from the cache directory.
    fn load_existing_entries(&mut self) -> Result<()> {
        // キャッシュディレクトリ内のファイルを列挙
        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| RenderError::Io(format!("Failed to read cache directory: {}", e)))?;

        // トータルサイズを計算
        let mut total_size = 0;

        // キャッシュのメタデータファイル名
        let metadata_path = self.cache_dir.join("cache_index.json");

        // メタデータが存在する場合は読み込む
        if metadata_path.exists() {
            let metadata_json = fs::read_to_string(&metadata_path)
                .map_err(|e| RenderError::Io(format!("Failed to read cache metadata: {}", e)))?;

            // JSONからキャッシュエントリを読み込む（簡略化のため実装を省略）
            // 実際の実装では、ここでJSONデシリアライズを行い、self.entriesを設定

            // 各ファイルのサイズをチェック
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    // メタデータファイルはスキップ
                    if path == metadata_path {
                        continue;
                    }

                    if let Ok(metadata) = fs::metadata(&path) {
                        total_size += metadata.len();
                    }
                }
            }
        }

        self.current_size = total_size;

        Ok(())
    }

    /// Saves the cache metadata to disk.
    fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.cache_dir.join("cache_index.json");

        // 簡略化のため、実際のJSON作成を省略
        // 実際の実装では、ここでself.entriesをJSONシリアライズして保存

        Ok(())
    }

    /// Generates a cache key for an asset and rendering parameters.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - ID of the source asset
    /// * `params` - Hash of the rendering parameters
    ///
    /// # Returns
    ///
    /// A cache key for the given asset and parameters.
    fn make_key(&self, asset_id: AssetId, params: u64) -> CacheKey {
        CacheKey {
            asset_id,
            params_hash: params,
        }
    }

    /// Calculates a hash for rendering parameters.
    ///
    /// # Arguments
    ///
    /// * `params` - The parameters to hash
    ///
    /// # Returns
    ///
    /// A hash value for the parameters.
    pub fn hash_params<T: std::hash::Hash>(&self, params: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        params.hash(&mut hasher);
        hasher.finish()
    }

    /// Gets a cached asset if available.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - ID of the source asset
    /// * `params_hash` - Hash of the rendering parameters
    ///
    /// # Returns
    ///
    /// The cache entry if found, or `None` if not found.
    pub fn get(&self, asset_id: AssetId, params_hash: u64) -> Option<&CacheEntry> {
        if !self.enabled {
            return None;
        }

        let key = self.make_key(asset_id, params_hash);
        self.entries.get(&key)
    }

    /// Adds a rendered asset to the cache.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - ID of the source asset
    /// * `params_hash` - Hash of the rendering parameters
    /// * `file_path` - Path to the rendered file
    /// * `duration` - Duration of the rendered content
    ///
    /// # Returns
    ///
    /// The path to the cached file, or an error if caching failed.
    pub fn add(
        &mut self,
        asset_id: AssetId,
        params_hash: u64,
        file_path: &Path,
        duration: Duration,
    ) -> Result<PathBuf> {
        if !self.enabled {
            return Ok(file_path.to_path_buf());
        }

        // ファイルのメタデータを取得
        let file_metadata = fs::metadata(file_path)
            .map_err(|e| RenderError::Io(format!("Failed to get file metadata: {}", e)))?;

        let file_size = file_metadata.len();

        // キャッシュキーを作成
        let key = self.make_key(asset_id, params_hash);

        // キャッシュファイルの名前を生成
        let cache_file_name = format!("asset_{}_{}.cache", asset_id, params_hash);
        let cache_path = self.cache_dir.join(cache_file_name);

        // ファイルをキャッシュにコピー
        fs::copy(file_path, &cache_path)
            .map_err(|e| RenderError::Io(format!("Failed to copy file to cache: {}", e)))?;

        // キャッシュメタデータを作成
        let metadata = CacheMetadata {
            created_at: SystemTime::now(),
            source_asset_id: asset_id,
            duration,
            params_hash,
            file_size,
        };

        // キャッシュエントリを作成
        let entry = CacheEntry {
            path: cache_path.clone(),
            metadata,
        };

        // キャッシュに追加
        self.entries.insert(key, entry);
        self.current_size += file_size;

        // キャッシュサイズが最大サイズを超えた場合、古いエントリを削除
        self.prune_if_needed();

        // メタデータ保存
        self.save_metadata()?;

        Ok(cache_path)
    }

    /// Prunes the cache if it's larger than the maximum size.
    fn prune_if_needed(&mut self) {
        if let Some(max_size) = self.max_size {
            if self.current_size > max_size {
                // 古い順にエントリをソート
                let mut entries_to_remove: Vec<_> = self.entries.iter().collect();
                entries_to_remove
                    .sort_by(|a, b| a.1.metadata.created_at.cmp(&b.1.metadata.created_at));

                // 削除するキーと容量を記録
                let mut keys_to_remove = Vec::new();
                let mut size_to_remove = 0u64;

                // キャッシュサイズが最大サイズ以下になるまで古いエントリを削除リストに追加
                for (key, entry) in &entries_to_remove {
                    keys_to_remove.push((*key).clone());
                    size_to_remove += entry.metadata.file_size;

                    // キャッシュサイズが最大サイズ以下になったか確認
                    if self.current_size - size_to_remove <= max_size {
                        break;
                    }
                }

                // 実際にエントリを削除
                for key in keys_to_remove {
                    if let Some(entry) = self.entries.remove(&key) {
                        // ファイルを削除
                        if let Err(e) = fs::remove_file(&entry.path) {
                            // エラーをログに記録（実際の実装ではロガーを使用）
                            eprintln!("Failed to remove cache file: {}", e);
                        }

                        // キャッシュサイズを更新
                        self.current_size -= entry.metadata.file_size;
                    }
                }
            }
        }
    }

    /// Invalidates a cached asset.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - ID of the source asset
    /// * `params_hash` - Optional hash of the rendering parameters (if None, all entries for the asset are invalidated)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the cache was invalidated, or an error if invalidation failed.
    pub fn invalidate(&mut self, asset_id: AssetId, params_hash: Option<u64>) -> Result<()> {
        // 削除するキーのリストを作成
        let keys_to_remove: Vec<CacheKey> = self
            .entries
            .keys()
            .filter(|k| {
                k.asset_id == asset_id
                    && (params_hash.is_none() || params_hash == Some(k.params_hash))
            })
            .cloned()
            .collect();

        // 該当するエントリを削除
        for key in keys_to_remove {
            if let Some(entry) = self.entries.remove(&key) {
                // ファイルを削除
                if let Err(e) = fs::remove_file(&entry.path) {
                    // エラーをログに記録（実際の実装ではロガーを使用）
                    eprintln!("Failed to remove cache file: {}", e);
                }

                // キャッシュサイズを更新
                self.current_size -= entry.metadata.file_size;
            }
        }

        // メタデータ保存
        self.save_metadata()?;

        Ok(())
    }

    /// Clears all cached assets.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the cache was cleared, or an error if clearing failed.
    pub fn clear(&mut self) -> Result<()> {
        // すべてのキャッシュファイルを削除
        for entry in &self.entries {
            if let Err(e) = fs::remove_file(&entry.1.path) {
                // エラーをログに記録（実際の実装ではロガーを使用）
                eprintln!("Failed to remove cache file: {}", e);
            }
        }

        // エントリをクリア
        self.entries.clear();
        self.current_size = 0;

        // メタデータ保存
        self.save_metadata()?;

        Ok(())
    }

    /// Enables or disables the cache.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether the cache should be enabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Checks if the cache is enabled.
    ///
    /// # Returns
    ///
    /// `true` if the cache is enabled, `false` otherwise.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Gets the current size of the cache in bytes.
    ///
    /// # Returns
    ///
    /// The current size of the cache.
    #[must_use]
    pub fn current_size(&self) -> u64 {
        self.current_size
    }

    /// Gets the maximum size of the cache in bytes.
    ///
    /// # Returns
    ///
    /// The maximum size of the cache, or `None` if unlimited.
    #[must_use]
    pub fn max_size(&self) -> Option<u64> {
        self.max_size
    }
}
