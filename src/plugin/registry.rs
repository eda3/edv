//! プラグインレジストリモジュール
//!
//! プラグインの登録、検索、およびプラグイン間の連携機能を提供します。
//! このモジュールは、プラグインタイプごとのレジストリやプラグイン検索機能を実装します。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use log::{debug, error, info, warn};

use super::host::PluginRegistryInterface;
use super::manager::PluginManager;
use super::types::{LoadedPlugin, Plugin, PluginState, PluginType};

/// プラグインレジストリ
///
/// プラグインの検索や連携を担当する拡張コンポーネント
pub struct PluginRegistry {
    /// プラグインマネージャへの参照
    manager: Arc<PluginManager>,
    /// プラグインタイプごとのレジストリ（プラグインID -> プラグイン情報）
    type_registries: RwLock<HashMap<PluginType, HashSet<String>>>,
    /// タグによるインデックス（タグ -> プラグインIDのセット）
    tag_index: RwLock<HashMap<String, HashSet<String>>>,
    /// カスタム機能のレジストリ（機能名 -> 実装プラグインIDのセット）
    feature_registry: RwLock<HashMap<String, HashSet<String>>>,
}

impl PluginRegistry {
    /// 新しいプラグインレジストリを作成
    pub fn new(manager: Arc<PluginManager>) -> Self {
        let registry = Self {
            manager,
            type_registries: RwLock::new(HashMap::new()),
            tag_index: RwLock::new(HashMap::new()),
            feature_registry: RwLock::new(HashMap::new()),
        };

        // 初期化
        registry.rebuild_indexes();

        registry
    }

    /// インデックスを再構築
    pub fn rebuild_indexes(&self) {
        // タイプレジストリをクリア
        let mut type_registries = self.type_registries.write().unwrap();
        type_registries.clear();

        // 各タイプの空のセットを初期化
        for plugin_type in [
            PluginType::Effect,
            PluginType::Exporter,
            PluginType::Importer,
            PluginType::UserInterface,
            PluginType::Custom,
        ]
        .iter()
        {
            type_registries.insert(*plugin_type, HashSet::new());
        }

        // すべてのプラグインを取得
        let plugins = self.manager.get_all_plugins();

        // アクティブなプラグインをタイプごとに分類
        for plugin in plugins {
            if plugin.state == PluginState::Active {
                if let Some(type_set) = type_registries.get_mut(&plugin.metadata.plugin_type) {
                    type_set.insert(plugin.metadata.id.clone());
                }
            }
        }

        // タグインデックスも再構築
        drop(type_registries); // ロックを解放
        self.rebuild_tag_index();

        // 機能レジストリも再構築
        self.rebuild_feature_registry();
    }

    /// タグインデックスを再構築
    fn rebuild_tag_index(&self) {
        let mut tag_index = self.tag_index.write().unwrap();
        tag_index.clear();

        // すべてのプラグインを取得
        let plugins = self.manager.get_all_plugins();

        // アクティブなプラグインのタグをインデックス化
        for plugin in plugins {
            if plugin.state == PluginState::Active {
                // 実際のアプリケーションでは、プラグインマニフェストなどからタグを取得
                // ここでは簡略化のため、タグはプラグインIDから抽出
                let tags = self.extract_tags_from_plugin_id(&plugin.metadata.id);

                for tag in tags {
                    let plugin_set = tag_index.entry(tag).or_insert_with(HashSet::new);
                    plugin_set.insert(plugin.metadata.id.clone());
                }
            }
        }
    }

    /// プラグインIDからタグを抽出する簡易実装
    fn extract_tags_from_plugin_id(&self, plugin_id: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // プラグインIDをドット（.）で分割し、各部分をタグとして扱う
        for part in plugin_id.split('.') {
            tags.push(part.to_lowercase());
        }

        tags
    }

    /// 機能レジストリを再構築
    fn rebuild_feature_registry(&self) {
        let mut feature_registry = self.feature_registry.write().unwrap();
        feature_registry.clear();

        // アクティブなプラグインを取得して、各プラグインの機能を登録
        // 実際のアプリケーションでは、プラグインから機能リストを取得する方法が必要
        // ここでは簡略化のため、実装を省略
    }

    /// 特定タイプのプラグインをすべて取得
    pub fn get_plugins_by_type(&self, plugin_type: PluginType) -> Vec<String> {
        let type_registries = self.type_registries.read().unwrap();

        if let Some(plugin_set) = type_registries.get(&plugin_type) {
            plugin_set.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// タグでプラグインを検索
    pub fn find_plugins_by_tag(&self, tag: &str) -> Vec<String> {
        let tag_index = self.tag_index.read().unwrap();

        if let Some(plugin_set) = tag_index.get(&tag.to_lowercase()) {
            plugin_set.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 複数タグでプラグインを検索（AND条件）
    pub fn find_plugins_by_tags(&self, tags: &[&str]) -> Vec<String> {
        if tags.is_empty() {
            return Vec::new();
        }

        let tag_index = self.tag_index.read().unwrap();

        // 最初のタグに一致するプラグインセットを取得
        let first_tag = tags[0].to_lowercase();
        let mut result_set = if let Some(plugin_set) = tag_index.get(&first_tag) {
            plugin_set.clone()
        } else {
            return Vec::new();
        };

        // 残りのタグで絞り込み
        for tag in tags.iter().skip(1) {
            let tag = tag.to_lowercase();
            if let Some(plugin_set) = tag_index.get(&tag) {
                // 交差（共通部分）を取得
                result_set = result_set.intersection(plugin_set).cloned().collect();

                if result_set.is_empty() {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        }

        result_set.into_iter().collect()
    }

    /// 特定の機能を提供するプラグインを検索
    pub fn find_plugins_by_feature(&self, feature_name: &str) -> Vec<String> {
        let feature_registry = self.feature_registry.read().unwrap();

        if let Some(plugin_set) = feature_registry.get(feature_name) {
            plugin_set.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// プラグインインスタンスを取得
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        self.manager.get_plugin(plugin_id)
    }

    /// 新しいプラグインが追加されたときの処理
    pub fn plugin_added(&self, plugin_id: &str) {
        info!("プラグインがレジストリに追加されました: {}", plugin_id);
        self.rebuild_indexes();
    }

    /// プラグインが削除されたときの処理
    pub fn plugin_removed(&self, plugin_id: &str) {
        info!("プラグインがレジストリから削除されました: {}", plugin_id);
        self.rebuild_indexes();
    }

    /// プラグインが更新されたときの処理
    pub fn plugin_updated(&self, plugin_id: &str) {
        info!("プラグインが更新されました: {}", plugin_id);
        self.rebuild_indexes();
    }

    /// 機能をレジストリに登録
    pub fn register_feature(&self, plugin_id: &str, feature_name: &str) {
        let mut feature_registry = self.feature_registry.write().unwrap();

        let plugin_set = feature_registry
            .entry(feature_name.to_string())
            .or_insert_with(HashSet::new);
        plugin_set.insert(plugin_id.to_string());

        debug!(
            "プラグイン {} が機能 {} を登録しました",
            plugin_id, feature_name
        );
    }

    /// 機能の登録を解除
    pub fn unregister_feature(&self, plugin_id: &str, feature_name: &str) {
        let mut feature_registry = self.feature_registry.write().unwrap();

        if let Some(plugin_set) = feature_registry.get_mut(feature_name) {
            plugin_set.remove(plugin_id);

            // セットが空になったら、エントリ自体を削除
            if plugin_set.is_empty() {
                feature_registry.remove(feature_name);
            }

            debug!(
                "プラグイン {} の機能 {} 登録を解除しました",
                plugin_id, feature_name
            );
        }
    }

    /// プラグインの依存関係を解決
    pub fn resolve_dependencies(&self, plugin_id: &str) -> Result<Vec<String>, String> {
        // プラグインの依存関係を取得
        // 実際のアプリケーションでは、マニフェストから依存関係を取得
        // ここでは簡略化のため、実装を省略

        Ok(Vec::new())
    }
}

impl PluginRegistryInterface for PluginRegistry {
    fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        self.get_plugin(plugin_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::manager::PluginInitOptions;
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    // モックプラグインの依存関係とタイプを設定するヘルパー関数
    fn setup_test_registry() -> Arc<PluginRegistry> {
        let manager = Arc::new(PluginManager::new(PluginInitOptions::default()).unwrap());
        let registry = Arc::new(PluginRegistry::new(manager.clone()));

        // 手動でタイプレジストリを設定
        let mut type_registries = registry.type_registries.write().unwrap();

        // エフェクトプラグイン
        let mut effects = HashSet::new();
        effects.insert("com.example.effect1".to_string());
        effects.insert("com.example.effect2".to_string());
        type_registries.insert(PluginType::Effect, effects);

        // エクスポータープラグイン
        let mut exporters = HashSet::new();
        exporters.insert("com.example.exporter1".to_string());
        type_registries.insert(PluginType::Exporter, exporters);

        // インポータープラグイン
        let mut importers = HashSet::new();
        importers.insert("org.test.importer1".to_string());
        importers.insert("org.test.importer2".to_string());
        type_registries.insert(PluginType::Importer, importers);

        drop(type_registries);

        // タグインデックスを手動で設定
        let mut tag_index = registry.tag_index.write().unwrap();

        // "com" タグを持つプラグイン
        let mut com_plugins = HashSet::new();
        com_plugins.insert("com.example.effect1".to_string());
        com_plugins.insert("com.example.effect2".to_string());
        com_plugins.insert("com.example.exporter1".to_string());
        tag_index.insert("com".to_string(), com_plugins);

        // "example" タグを持つプラグイン
        let mut example_plugins = HashSet::new();
        example_plugins.insert("com.example.effect1".to_string());
        example_plugins.insert("com.example.effect2".to_string());
        example_plugins.insert("com.example.exporter1".to_string());
        tag_index.insert("example".to_string(), example_plugins);

        // "org" タグを持つプラグイン
        let mut org_plugins = HashSet::new();
        org_plugins.insert("org.test.importer1".to_string());
        org_plugins.insert("org.test.importer2".to_string());
        tag_index.insert("org".to_string(), org_plugins);

        // "test" タグを持つプラグイン
        let mut test_plugins = HashSet::new();
        test_plugins.insert("org.test.importer1".to_string());
        test_plugins.insert("org.test.importer2".to_string());
        tag_index.insert("test".to_string(), test_plugins);

        registry
    }

    #[test]
    fn test_get_plugins_by_type() {
        let registry = setup_test_registry();

        // エフェクトプラグインを取得
        let effects = registry.get_plugins_by_type(PluginType::Effect);
        assert_eq!(effects.len(), 2);
        assert!(effects.contains(&"com.example.effect1".to_string()));
        assert!(effects.contains(&"com.example.effect2".to_string()));

        // エクスポータープラグインを取得
        let exporters = registry.get_plugins_by_type(PluginType::Exporter);
        assert_eq!(exporters.len(), 1);
        assert!(exporters.contains(&"com.example.exporter1".to_string()));

        // インポータープラグインを取得
        let importers = registry.get_plugins_by_type(PluginType::Importer);
        assert_eq!(importers.len(), 2);
        assert!(importers.contains(&"org.test.importer1".to_string()));
        assert!(importers.contains(&"org.test.importer2".to_string()));

        // UIプラグインを取得（存在しない）
        let ui_plugins = registry.get_plugins_by_type(PluginType::UserInterface);
        assert!(ui_plugins.is_empty());
    }

    #[test]
    fn test_find_plugins_by_tag() {
        let registry = setup_test_registry();

        // "com" タグを持つプラグインを検索
        let com_plugins = registry.find_plugins_by_tag("com");
        assert_eq!(com_plugins.len(), 3);
        assert!(com_plugins.contains(&"com.example.effect1".to_string()));
        assert!(com_plugins.contains(&"com.example.effect2".to_string()));
        assert!(com_plugins.contains(&"com.example.exporter1".to_string()));

        // "org" タグを持つプラグインを検索
        let org_plugins = registry.find_plugins_by_tag("org");
        assert_eq!(org_plugins.len(), 2);
        assert!(org_plugins.contains(&"org.test.importer1".to_string()));
        assert!(org_plugins.contains(&"org.test.importer2".to_string()));

        // 存在しないタグで検索
        let unknown_plugins = registry.find_plugins_by_tag("unknown");
        assert!(unknown_plugins.is_empty());
    }

    #[test]
    fn test_find_plugins_by_tags() {
        let registry = setup_test_registry();

        // "com" AND "example" タグを持つプラグインを検索
        let com_example_plugins = registry.find_plugins_by_tags(&["com", "example"]);
        assert_eq!(com_example_plugins.len(), 3);
        assert!(com_example_plugins.contains(&"com.example.effect1".to_string()));
        assert!(com_example_plugins.contains(&"com.example.effect2".to_string()));
        assert!(com_example_plugins.contains(&"com.example.exporter1".to_string()));

        // "org" AND "test" タグを持つプラグインを検索
        let org_test_plugins = registry.find_plugins_by_tags(&["org", "test"]);
        assert_eq!(org_test_plugins.len(), 2);
        assert!(org_test_plugins.contains(&"org.test.importer1".to_string()));
        assert!(org_test_plugins.contains(&"org.test.importer2".to_string()));

        // "com" AND "test" タグを持つプラグインを検索（存在しない）
        let com_test_plugins = registry.find_plugins_by_tags(&["com", "test"]);
        assert!(com_test_plugins.is_empty());
    }
}
