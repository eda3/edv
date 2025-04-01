//! プラグインマネージャーモジュール
//!
//! プラグインシステムの中心的な管理コンポーネントです。
//! プラグインの検出、ロード、初期化、実行、シャットダウン、アンロードなど
//! ライフサイクル全体を管理します。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

use log::{debug, error, info, warn};

use super::host::{HostFactory, HostImpl, PluginHostInstance, PluginRegistryInterface};
use super::loader::PluginLoader;
use super::manifest::{extract_dependencies, get_entry_point, load_manifest, validate_manifest};
use super::security::PermissionChecker;
use super::types::{LoadedPlugin, Plugin, PluginMetadata, PluginState, PluginType};
use super::{PluginDependency, PluginError, PluginResult};

/// プラグインのイベントハンドラ
pub type PluginEventHandler = Box<dyn Fn(&str, PluginState) + Send + Sync>;

/// プラグイン依存関係グラフ
type DependencyGraph = HashMap<String, Vec<String>>;

/// プラグインマネージャー
pub struct PluginManager {
    /// プラグインローダー
    loader: Mutex<PluginLoader>,
    /// ホスト実装
    host: Arc<HostImpl>,
    /// ロードされたプラグインのマップ (ID → LoadedPlugin)
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    /// プラグインの依存関係グラフ
    dependencies: RwLock<DependencyGraph>,
    /// イベントハンドラのリスト
    event_handlers: RwLock<Vec<PluginEventHandler>>,
    /// プラグインディレクトリの監視間隔
    watch_interval: Duration,
    /// ユーザーデータディレクトリ
    user_data_dir: PathBuf,
    /// 権限チェッカー
    permission_checker: Arc<PermissionChecker>,
}

/// プラグインの初期化オプション
#[derive(Debug, Default)]
pub struct PluginInitOptions {
    /// 自動的にプラグインをロードするか
    pub auto_load: bool,
    /// 信頼されたプラグインの自動初期化
    pub auto_initialize_trusted: bool,
    /// デフォルトで有効にするプラグインタイプ
    pub enabled_types: Option<HashSet<PluginType>>,
    /// デフォルトで無効にするプラグインID
    pub disabled_plugins: Option<HashSet<String>>,
    /// ホットリロードを有効にするか
    pub enable_hot_reload: bool,
    /// プラグインディレクトリの監視間隔（秒）
    pub watch_interval_seconds: Option<u64>,
    /// プラグイン検索パス（追加）
    pub additional_search_paths: Option<Vec<PathBuf>>,
}

impl PluginManager {
    /// 新しいプラグインマネージャーを作成
    pub fn new(options: PluginInitOptions) -> PluginResult<Self> {
        let mut loader = PluginLoader::new();

        // システムプラグインディレクトリを追加
        for dir in super::get_system_plugin_dirs() {
            loader.add_search_path(dir);
        }

        // ユーザープラグインディレクトリを追加
        let user_plugin_dir = super::get_user_plugin_dir()?;
        loader.add_search_path(user_plugin_dir.clone());

        // 追加のプラグイン検索パスを追加
        if let Some(additional_paths) = options.additional_search_paths {
            for path in additional_paths {
                loader.add_search_path(path);
            }
        }

        // 監視間隔を設定
        let watch_interval = Duration::from_secs(options.watch_interval_seconds.unwrap_or(60));

        // 権限チェッカーを作成
        let permission_checker = Arc::new(PermissionChecker::new());

        // ホストを作成
        let host = HostFactory::create_default_host();

        let manager = Self {
            loader: Mutex::new(loader),
            host,
            plugins: RwLock::new(HashMap::new()),
            dependencies: RwLock::new(HashMap::new()),
            event_handlers: RwLock::new(Vec::new()),
            watch_interval,
            user_data_dir: user_plugin_dir,
            permission_checker: permission_checker.clone(),
        };

        // 自動ロードが有効なら、プラグインを検出してロード
        if options.auto_load {
            manager.discover_plugins()?;

            if options.auto_initialize_trusted {
                manager.initialize_all_plugins()?;
            }
        }

        Ok(manager)
    }

    /// プラグインの検出
    pub fn discover_plugins(&self) -> PluginResult<Vec<String>> {
        info!("プラグインの検出を開始");

        let mut discovered_ids = Vec::new();
        let loader = self.loader.lock().unwrap();
        let plugin_paths = loader.discover_plugins();

        // 各マニフェストを検証
        for (manifest_path, lib_path) in plugin_paths {
            let validation = validate_manifest(&manifest_path);

            if !validation.is_valid {
                for error in &validation.errors {
                    error!(
                        "マニフェスト検証エラー ({}): {}",
                        manifest_path.display(),
                        error
                    );
                }
                continue;
            }

            for warning in &validation.warnings {
                warn!(
                    "マニフェスト検証警告 ({}): {}",
                    manifest_path.display(),
                    warning
                );
            }

            // マニフェストをロード
            match load_manifest(&manifest_path) {
                Ok(manifest) => {
                    let plugin_id = manifest.plugin.id.clone();

                    // プラグインの依存関係を取得
                    let dependencies = extract_dependencies(&manifest);
                    self.register_dependencies(&plugin_id, &dependencies);

                    // 循環依存関係をチェック
                    if self.has_circular_dependencies(&plugin_id) {
                        error!("プラグイン {} に循環依存関係が検出されました", plugin_id);
                        continue;
                    }

                    // メッセージ
                    info!("プラグインを検出: {} ({})", manifest.plugin.name, plugin_id);

                    // プラグインをロード
                    match loader.load_plugin(&manifest_path, &lib_path) {
                        Ok(mut loaded_plugin) => {
                            // 権限初期設定
                            self.permission_checker.register_plugin(&plugin_id);

                            // プラグインデータディレクトリへのアクセス権を付与
                            let data_dir = self.user_data_dir.join(&plugin_id);
                            self.permission_checker
                                .grant_data_directory_access(&plugin_id, &data_dir);

                            // プラグインを記録
                            loaded_plugin.state = PluginState::Discovered;

                            let mut plugins = self.plugins.write().unwrap();
                            plugins.insert(plugin_id.clone(), loaded_plugin);

                            discovered_ids.push(plugin_id);
                        }
                        Err(e) => {
                            error!("プラグイン {} のロードに失敗: {}", plugin_id, e);
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "マニフェスト {} の読み込みに失敗: {}",
                        manifest_path.display(),
                        e
                    );
                }
            }
        }

        info!(
            "プラグインの検出完了: {} 個見つかりました",
            discovered_ids.len()
        );

        Ok(discovered_ids)
    }

    /// 全プラグインの初期化
    pub fn initialize_all_plugins(&self) -> PluginResult<Vec<String>> {
        let mut initialized_ids = Vec::new();
        let plugins = self.plugins.read().unwrap();

        // 初期化対象のプラグインIDを収集
        let plugin_ids: Vec<String> = plugins
            .iter()
            .filter(|(_, p)| p.state == PluginState::Discovered || p.state == PluginState::Loaded)
            .map(|(id, _)| id.clone())
            .collect();

        // 依存関係を考慮した初期化順序を取得
        let ordered_ids = self.get_initialization_order(&plugin_ids)?;

        // 順序に従ってプラグインを初期化
        for id in ordered_ids {
            match self.initialize_plugin(&id) {
                Ok(_) => {
                    initialized_ids.push(id);
                }
                Err(e) => {
                    error!("プラグイン {} の初期化に失敗: {}", id, e);
                }
            }
        }

        info!(
            "プラグインの初期化完了: {} 個初期化されました",
            initialized_ids.len()
        );

        Ok(initialized_ids)
    }

    /// 単一プラグインの初期化
    pub fn initialize_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().unwrap();

        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        // すでにアクティブならスキップ
        if plugin.state == PluginState::Active {
            return Ok(());
        }

        // ロードされていなければエラー
        if plugin.state != PluginState::Discovered && plugin.state != PluginState::Loaded {
            return Err(PluginError::InvalidState(format!(
                "プラグイン {} は現在 {:?} 状態です。初期化には Discovered または Loaded 状態が必要です。",
                plugin_id, plugin.state
            )));
        }

        info!("プラグイン {} の初期化を開始", plugin_id);

        // マニフェストからエントリーポイントを取得
        let manifest = load_manifest(&plugin.manifest_path)?;
        let entry_point = get_entry_point(&manifest);

        // プラグインをインスタンス化
        let loader = self.loader.lock().unwrap();
        let plugin_instance = loader.instantiate_plugin(plugin_id, entry_point)?;

        // ホストインスタンスを作成
        let host_instance = Box::new(PluginHostInstance::new(
            plugin_id.to_string(),
            self.host.clone(),
        ));

        // プラグインの状態を更新
        drop(plugins); // ロックを解放（initialize内でプラグインマネージャーを使う可能性があるため）

        // プラグインを初期化
        let mut plugin_instance = plugin_instance;
        match plugin_instance.initialize(host_instance) {
            Ok(_) => {
                // 成功したらインスタンスを保存し、状態をアクティブに
                let mut plugins = self.plugins.write().unwrap();
                if let Some(plugin) = plugins.get_mut(plugin_id) {
                    plugin.instance = Some(plugin_instance);
                    plugin.state = PluginState::Active;
                    plugin.error = None;

                    // プラグイン状態変更イベントを発火
                    drop(plugins); // ロックを解放
                    self.fire_plugin_event(plugin_id, PluginState::Active);

                    info!("プラグイン {} の初期化が完了しました", plugin_id);
                    Ok(())
                } else {
                    Err(PluginError::PluginNotFound(plugin_id.to_string()))
                }
            }
            Err(e) => {
                // 失敗したら状態をエラーに
                let mut plugins = self.plugins.write().unwrap();
                if let Some(plugin) = plugins.get_mut(plugin_id) {
                    plugin.state = PluginState::Error;
                    plugin.error = Some(e.to_string());

                    // プラグイン状態変更イベントを発火
                    drop(plugins); // ロックを解放
                    self.fire_plugin_event(plugin_id, PluginState::Error);
                }

                error!("プラグイン {} の初期化に失敗: {}", plugin_id, e);
                Err(e)
            }
        }
    }

    /// プラグインのシャットダウン
    pub fn shutdown_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().unwrap();

        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        // すでに非アクティブならスキップ
        if plugin.state != PluginState::Active {
            return Ok(());
        }

        info!("プラグイン {} のシャットダウンを開始", plugin_id);

        // インスタンスを取得
        if let Some(instance) = plugin.instance.as_mut() {
            // シャットダウン処理
            match instance.shutdown() {
                Ok(_) => {
                    plugin.state = PluginState::Inactive;

                    // プラグイン状態変更イベントを発火
                    drop(plugins); // ロックを解放
                    self.fire_plugin_event(plugin_id, PluginState::Inactive);

                    info!("プラグイン {} のシャットダウンが完了しました", plugin_id);
                    Ok(())
                }
                Err(e) => {
                    // エラー状態に
                    plugin.state = PluginState::Error;
                    plugin.error = Some(e.to_string());

                    // プラグイン状態変更イベントを発火
                    drop(plugins); // ロックを解放
                    self.fire_plugin_event(plugin_id, PluginState::Error);

                    error!("プラグイン {} のシャットダウンに失敗: {}", plugin_id, e);
                    Err(e)
                }
            }
        } else {
            Err(PluginError::InvalidState(format!(
                "プラグイン {} のインスタンスが存在しません",
                plugin_id
            )))
        }
    }

    /// 全プラグインのシャットダウン
    pub fn shutdown_all_plugins(&self) -> PluginResult<Vec<String>> {
        let mut shutdown_ids = Vec::new();
        let plugins = self.plugins.read().unwrap();

        // シャットダウン対象のプラグインIDを収集 (依存関係の逆順)
        let plugin_ids: Vec<String> = plugins
            .iter()
            .filter(|(_, p)| p.state == PluginState::Active)
            .map(|(id, _)| id.clone())
            .collect();

        drop(plugins); // ロックを解放

        // 依存関係を考慮した逆順で処理
        let reversed_order = self.get_reverse_initialization_order(&plugin_ids)?;

        // 順序に従ってシャットダウン
        for id in reversed_order {
            match self.shutdown_plugin(&id) {
                Ok(_) => {
                    shutdown_ids.push(id);
                }
                Err(e) => {
                    error!("プラグイン {} のシャットダウンに失敗: {}", id, e);
                }
            }
        }

        info!(
            "全プラグインのシャットダウン完了: {} 個シャットダウンされました",
            shutdown_ids.len()
        );

        Ok(shutdown_ids)
    }

    /// プラグインのアンロード
    pub fn unload_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        // まずシャットダウンを試みる
        let _ = self.shutdown_plugin(plugin_id);

        let mut plugins = self.plugins.write().unwrap();

        if let Some(plugin) = plugins.remove(plugin_id) {
            // ライブラリをアンロード
            let mut loader = self.loader.lock().unwrap();
            let _ = loader.unload_plugin(plugin_id);

            // 権限システムから削除
            self.permission_checker.unregister_plugin(plugin_id);

            // 依存関係グラフから削除
            let mut dependencies = self.dependencies.write().unwrap();
            dependencies.remove(plugin_id);
            for (_, deps) in dependencies.iter_mut() {
                deps.retain(|id| id != plugin_id);
            }

            info!("プラグイン {} をアンロードしました", plugin_id);
            Ok(())
        } else {
            Err(PluginError::PluginNotFound(plugin_id.to_string()))
        }
    }

    /// 全プラグインのアンロード
    pub fn unload_all_plugins(&self) -> PluginResult<Vec<String>> {
        // まず全てをシャットダウン
        let _ = self.shutdown_all_plugins();

        let mut unloaded_ids = Vec::new();
        let mut plugins = self.plugins.write().unwrap();

        // 全プラグインIDを取得
        let ids: Vec<String> = plugins.keys().cloned().collect();

        // 1つずつアンロード
        for id in &ids {
            if let Some(plugin) = plugins.remove(id) {
                unloaded_ids.push(id.clone());
            }
        }

        // 依存関係グラフをクリア
        let mut dependencies = self.dependencies.write().unwrap();
        dependencies.clear();

        // ライブラリをすべてアンロード
        let mut loader = self.loader.lock().unwrap();
        loader.unload_all_plugins();

        info!(
            "全プラグインのアンロード完了: {} 個アンロードされました",
            unloaded_ids.len()
        );

        Ok(unloaded_ids)
    }

    /// プラグインの再ロード
    pub fn reload_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        // まずアンロード
        let _ = self.unload_plugin(plugin_id);

        // 再検出
        self.discover_plugins()?;

        // 再初期化
        let plugins = self.plugins.read().unwrap();
        if plugins.contains_key(plugin_id) {
            drop(plugins); // ロックを解放
            self.initialize_plugin(plugin_id)
        } else {
            Err(PluginError::PluginNotFound(plugin_id.to_string()))
        }
    }

    /// 特定タイプのプラグインを取得
    pub fn get_plugins_by_type(&self, plugin_type: PluginType) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();

        plugins
            .iter()
            .filter(|(_, p)| {
                p.metadata.plugin_type == plugin_type && p.state == PluginState::Active
            })
            .map(|(_, p)| p.metadata.clone())
            .collect()
    }

    /// プラグインの情報を取得
    pub fn get_plugin_info(&self, plugin_id: &str) -> Option<LoadedPlugin> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(plugin_id).cloned()
    }

    /// 全プラグインの情報を取得
    pub fn get_all_plugins(&self) -> Vec<LoadedPlugin> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().cloned().collect()
    }

    /// プラグインのインスタンスを取得
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        let plugins = self.plugins.read().unwrap();
        if let Some(plugin) = plugins.get(plugin_id) {
            if plugin.state == PluginState::Active {
                if let Some(instance) = &plugin.instance {
                    return Some(instance.as_ref());
                }
            }
        }
        None
    }

    /// プラグインイベントハンドラを登録
    pub fn register_event_handler(&self, handler: PluginEventHandler) {
        let mut handlers = self.event_handlers.write().unwrap();
        handlers.push(handler);
    }

    /// 依存関係を登録
    fn register_dependencies(&self, plugin_id: &str, dependencies: &[PluginDependency]) {
        let mut deps_map = self.dependencies.write().unwrap();

        let deps = deps_map
            .entry(plugin_id.to_string())
            .or_insert_with(Vec::new);
        for dep in dependencies {
            if !deps.contains(&dep.id) {
                deps.push(dep.id.clone());
            }
        }
    }

    /// 循環依存関係をチェック
    fn has_circular_dependencies(&self, plugin_id: &str) -> bool {
        let deps_map = self.dependencies.read().unwrap();
        let mut visited = HashSet::new();
        let mut path = HashSet::new();

        // DFSで循環依存関係を検出
        fn dfs(
            plugin_id: &str,
            deps_map: &DependencyGraph,
            visited: &mut HashSet<String>,
            path: &mut HashSet<String>,
        ) -> bool {
            if path.contains(plugin_id) {
                return true; // 循環検出
            }

            if visited.contains(plugin_id) {
                return false; // すでに訪問済み
            }

            visited.insert(plugin_id.to_string());
            path.insert(plugin_id.to_string());

            if let Some(deps) = deps_map.get(plugin_id) {
                for dep in deps {
                    if dfs(dep, deps_map, visited, path) {
                        return true;
                    }
                }
            }

            path.remove(plugin_id);
            false
        }

        dfs(plugin_id, &deps_map, &mut visited, &mut path)
    }

    /// 初期化順序を取得（依存関係を考慮）
    fn get_initialization_order(&self, plugin_ids: &[String]) -> PluginResult<Vec<String>> {
        let deps_map = self.dependencies.read().unwrap();
        let mut result = Vec::new();
        let mut visited = HashSet::new();

        // トポロジカルソートを行う
        fn visit(
            plugin_id: &str,
            deps_map: &DependencyGraph,
            visited: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) {
            if visited.contains(plugin_id) {
                return;
            }

            visited.insert(plugin_id.to_string());

            if let Some(deps) = deps_map.get(plugin_id) {
                for dep in deps {
                    visit(dep, deps_map, visited, result);
                }
            }

            result.push(plugin_id.to_string());
        }

        for id in plugin_ids {
            visit(id, &deps_map, &mut visited, &mut result);
        }

        Ok(result)
    }

    /// 逆初期化順序を取得（依存関係の逆順）
    fn get_reverse_initialization_order(&self, plugin_ids: &[String]) -> PluginResult<Vec<String>> {
        let mut ordered = self.get_initialization_order(plugin_ids)?;
        ordered.reverse();
        Ok(ordered)
    }

    /// プラグインイベントを発火
    fn fire_plugin_event(&self, plugin_id: &str, state: PluginState) {
        let handlers = self.event_handlers.read().unwrap();
        for handler in &*handlers {
            handler(plugin_id, state);
        }
    }
}

impl PluginRegistryInterface for PluginManager {
    fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        self.get_plugin(plugin_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_initialization_order() {
        // 依存関係グラフを作成
        // A → B → C
        // ↓
        // D
        let mut manager = PluginManager::new(PluginInitOptions::default()).unwrap();

        let mut deps_map = manager.dependencies.write().unwrap();
        deps_map.insert("A".to_string(), vec!["B".to_string(), "D".to_string()]);
        deps_map.insert("B".to_string(), vec!["C".to_string()]);
        deps_map.insert("C".to_string(), vec![]);
        deps_map.insert("D".to_string(), vec![]);
        drop(deps_map);

        let plugin_ids = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ];
        let order = manager.get_initialization_order(&plugin_ids).unwrap();

        // 各プラグインが依存するプラグインの後になっていることを確認
        let c_pos = order.iter().position(|id| id == "C").unwrap();
        let b_pos = order.iter().position(|id| id == "B").unwrap();
        let d_pos = order.iter().position(|id| id == "D").unwrap();
        let a_pos = order.iter().position(|id| id == "A").unwrap();

        assert!(c_pos < b_pos); // Cが先にくる
        assert!(b_pos < a_pos); // Bが先にくる
        assert!(d_pos < a_pos); // Dが先にくる
    }

    #[test]
    fn test_circular_dependencies() {
        let mut manager = PluginManager::new(PluginInitOptions::default()).unwrap();

        // 循環依存関係を作成
        // A → B → C → A
        let mut deps_map = manager.dependencies.write().unwrap();
        deps_map.insert("A".to_string(), vec!["B".to_string()]);
        deps_map.insert("B".to_string(), vec!["C".to_string()]);
        deps_map.insert("C".to_string(), vec!["A".to_string()]);
        drop(deps_map);

        // 循環が検出されることを確認
        assert!(manager.has_circular_dependencies("A"));
        assert!(manager.has_circular_dependencies("B"));
        assert!(manager.has_circular_dependencies("C"));
    }

    #[test]
    fn test_event_handler() {
        let manager = PluginManager::new(PluginInitOptions::default()).unwrap();

        // イベントが発火されたかのフラグ
        let event_fired = Arc::new(AtomicBool::new(false));
        let event_fired_clone = event_fired.clone();

        // イベントハンドラを登録
        manager.register_event_handler(Box::new(move |plugin_id, state| {
            if plugin_id == "test" && state == PluginState::Active {
                event_fired_clone.store(true, Ordering::SeqCst);
            }
        }));

        // イベントを発火
        manager.fire_plugin_event("test", PluginState::Active);

        // イベントが発火されたことを確認
        assert!(event_fired.load(Ordering::SeqCst));
    }
}
