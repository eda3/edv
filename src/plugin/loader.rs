//! プラグインローダーモジュール
//!
//! プラグインの共有ライブラリ（.so、.dll、.dylib）を動的にロードし、
//! エントリーポイント関数を呼び出して、プラグインインスタンスを取得する機能を提供します。

use std::collections::HashMap;
use std::ffi::OsStr;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libloading::{Library, Symbol};
use log::{debug, error, warn};

use super::PluginError;
use super::manifest::{extract_capabilities, extract_metadata, get_entry_point, load_manifest};
use super::types::{LoadedPlugin, Plugin, PluginMetadata, PluginState, PluginType};

/// プラグインエントリーポイント関数のシグネチャ
pub type PluginEntryFn = unsafe fn() -> *mut dyn Plugin;

/// プラグインライブラリのラッパー
pub struct PluginLibrary {
    /// ライブラリハンドル
    library: Library,
    /// ライブラリパス
    path: PathBuf,
}

impl PluginLibrary {
    /// ライブラリをロードする
    fn load(path: &Path) -> Result<Self, PluginError> {
        let library = unsafe {
            Library::new(path).map_err(|e| {
                PluginError::LoadingFailed(format!(
                    "ライブラリのロードに失敗: {}: {}",
                    path.display(),
                    e
                ))
            })?
        };

        Ok(Self {
            library,
            path: path.to_path_buf(),
        })
    }

    /// エントリーポイント関数を取得
    fn get_entry_point(&self, symbol_name: &str) -> Result<PluginEntryFn, PluginError> {
        unsafe {
            let symbol: Symbol<PluginEntryFn> =
                self.library.get(symbol_name.as_bytes()).map_err(|e| {
                    PluginError::LoadingFailed(format!(
                        "エントリーポイント関数の取得に失敗: {}: {}",
                        symbol_name, e
                    ))
                })?;

            Ok(*symbol)
        }
    }

    /// プラグインインスタンスを作成
    fn create_plugin(&self, entry_point_name: &str) -> Result<Box<dyn Plugin>, PluginError> {
        let entry_point = self.get_entry_point(entry_point_name)?;

        let plugin = unsafe {
            let plugin_ptr = entry_point();
            if plugin_ptr.is_null() {
                return Err(PluginError::InitializationFailed(
                    "エントリーポイント関数がnullを返しました".to_string(),
                ));
            }

            Box::from_raw(plugin_ptr)
        };

        Ok(plugin)
    }
}

/// プラグインローダー
///
/// プラグインの検出、ロード、初期化を担当します。
#[derive(Default)]
pub struct PluginLoader {
    /// ロードされたライブラリのキャッシュ (プラグインID -> ライブラリ)
    loaded_libraries: HashMap<String, Arc<PluginLibrary>>,
    /// プラグインパスの検索ディレクトリ
    search_paths: Vec<PathBuf>,
}

impl PluginLoader {
    /// 新しいプラグインローダーを作成
    pub fn new() -> Self {
        Self {
            loaded_libraries: HashMap::new(),
            search_paths: Vec::new(),
        }
    }

    /// 検索パスを追加
    pub fn add_search_path(&mut self, path: PathBuf) {
        if path.is_dir() && !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// 検索パスを取得
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// プラグインディレクトリ内のマニフェストを検出
    pub fn discover_plugins(&self) -> Vec<(PathBuf, PathBuf)> {
        let mut result = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() || !search_path.is_dir() {
                continue;
            }

            match std::fs::read_dir(search_path) {
                Ok(entries) => {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_dir() {
                            let manifest_path = path.join("plugin.toml");
                            if manifest_path.exists() && manifest_path.is_file() {
                                // ライブラリファイルを探す
                                if let Some(lib_path) = self.find_library_file(&path) {
                                    result.push((manifest_path, lib_path));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "ディレクトリの読み取りに失敗: {}: {}",
                        search_path.display(),
                        e
                    );
                }
            }
        }

        result
    }

    /// プラグインディレクトリからライブラリファイルを探す
    fn find_library_file(&self, plugin_dir: &Path) -> Option<PathBuf> {
        let lib_extensions = get_platform_lib_extensions();
        let dir_name = plugin_dir.file_name()?;
        let expected_lib_name = format!("lib{}", dir_name.to_string_lossy());

        for ext in lib_extensions {
            // 優先度1: libプラグイン名.拡張子
            let lib_path = plugin_dir.join(format!("{}.{}", expected_lib_name, ext));
            if lib_path.exists() && lib_path.is_file() {
                return Some(lib_path);
            }

            // 優先度2: プラグイン名.拡張子
            let lib_path = plugin_dir.join(format!("{}.{}", dir_name.to_string_lossy(), ext));
            if lib_path.exists() && lib_path.is_file() {
                return Some(lib_path);
            }
        }

        // どのパターンにも一致しなければ、任意の対応するライブラリファイルを探す
        if let Ok(entries) = std::fs::read_dir(plugin_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
                        if lib_extensions.contains(&ext) {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// プラットフォームに応じたライブラリ拡張子を取得
    fn get_platform_lib_extensions() -> Vec<&'static str> {
        if cfg!(target_os = "windows") {
            vec!["dll"]
        } else if cfg!(target_os = "macos") {
            vec!["dylib", "so"]
        } else {
            vec!["so"]
        }
    }

    /// プラグインをロード
    pub fn load_plugin(
        &mut self,
        manifest_path: &Path,
        lib_path: &Path,
    ) -> Result<LoadedPlugin, PluginError> {
        debug!("プラグインのロード: {}", manifest_path.display());

        // マニフェストを読み込む
        let manifest = load_manifest(manifest_path)?;

        // メタデータを抽出
        let metadata = extract_metadata(&manifest)?;

        // 機能を抽出
        let capabilities = extract_capabilities(&manifest);

        // エントリーポイント関数名を取得
        let entry_point = get_entry_point(&manifest);

        // ライブラリをロード
        let library = PluginLibrary::load(lib_path)?;

        let loaded_plugin = LoadedPlugin {
            metadata: metadata.clone(),
            state: PluginState::Loaded,
            capabilities,
            directory: manifest_path.parent().unwrap().to_path_buf(),
            library_path: lib_path.to_path_buf(),
            manifest_path: manifest_path.to_path_buf(),
            instance: None,
            error: None,
            load_time: std::time::SystemTime::now(),
        };

        // ライブラリをキャッシュに保存
        self.loaded_libraries
            .insert(metadata.id.clone(), Arc::new(library));

        Ok(loaded_plugin)
    }

    /// プラグインをインスタンス化
    pub fn instantiate_plugin(
        &self,
        plugin_id: &str,
        entry_point: &str,
    ) -> Result<Box<dyn Plugin>, PluginError> {
        debug!("プラグインのインスタンス化: {}", plugin_id);

        // ライブラリを取得
        let library = self
            .loaded_libraries
            .get(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        // プラグインインスタンスを作成
        library.create_plugin(entry_point)
    }

    /// プラグインをアンロード
    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        debug!("プラグインのアンロード: {}", plugin_id);

        if self.loaded_libraries.remove(plugin_id).is_none() {
            return Err(PluginError::PluginNotFound(plugin_id.to_string()));
        }

        // メモリリークを防ぐためにガベージコレクションを促進
        drop(mem::take(&mut self.loaded_libraries));

        Ok(())
    }

    /// 全てのプラグインをアンロード
    pub fn unload_all_plugins(&mut self) {
        debug!("全プラグインのアンロード");

        self.loaded_libraries.clear();

        // メモリリークを防ぐためにガベージコレクションを促進
        drop(mem::take(&mut self.loaded_libraries));
    }
}

/// プラットフォームに応じたライブラリ拡張子を取得
pub fn get_platform_lib_extensions() -> Vec<&'static str> {
    if cfg!(target_os = "windows") {
        vec!["dll"]
    } else if cfg!(target_os = "macos") {
        vec!["dylib", "so"]
    } else {
        vec!["so"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    // テスト用のダミープラグインディレクトリを作成
    fn create_test_plugin_dir() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // プラグインディレクトリを作成
        let plugin_dir = temp_dir.path().join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        // マニフェストファイルを作成
        let manifest_content = r#"
            [plugin]
            id = "test.plugin"
            name = "Test Plugin"
            version = "1.0.0"
            author = "Test Author"
            description = "A test plugin"
            type = "effect"
            api_version = "1.0"
            entry_point = "create_plugin"
            
            [capabilities]
            has_settings_ui = true
            thread_safe = true
        "#;

        let manifest_path = plugin_dir.join("plugin.toml");
        let mut file = File::create(&manifest_path).unwrap();
        file.write_all(manifest_content.as_bytes()).unwrap();

        // ダミーのライブラリファイルを作成（実際には動作しないダミー）
        let lib_ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        let lib_path = plugin_dir.join(format!("libtest-plugin.{}", lib_ext));
        let mut file = File::create(&lib_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        temp_dir
    }

    #[test]
    fn test_discover_plugins() {
        let temp_dir = create_test_plugin_dir();

        let mut loader = PluginLoader::new();
        loader.add_search_path(temp_dir.path().to_path_buf());

        let discovered = loader.discover_plugins();
        assert_eq!(discovered.len(), 1);

        let (manifest_path, lib_path) = &discovered[0];
        assert!(manifest_path.ends_with("plugin.toml"));

        let lib_ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        assert!(
            lib_path
                .to_string_lossy()
                .contains(&format!(".{}", lib_ext))
        );
    }

    #[test]
    fn test_add_search_path() {
        let temp_dir = create_test_plugin_dir();
        let path = temp_dir.path().to_path_buf();

        let mut loader = PluginLoader::new();
        loader.add_search_path(path.clone());

        assert_eq!(loader.search_paths().len(), 1);
        assert_eq!(loader.search_paths()[0], path);

        // 同じパスを追加しても重複しないことを確認
        loader.add_search_path(path.clone());
        assert_eq!(loader.search_paths().len(), 1);

        // 存在しないディレクトリは追加されないことを確認
        loader.add_search_path(path.join("nonexistent"));
        assert_eq!(loader.search_paths().len(), 1);
    }

    // 注: ライブラリのロードやプラグインのインスタンス化は、実際のダイナミックライブラリが
    // 必要なため、ここではテストしません。それらは統合テストでカバーする必要があります。
}
