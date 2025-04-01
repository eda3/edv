//! サンプルインポータープラグイン実装
//!
//! このモジュールでは、EDVのプラグインシステムを使った
//! PSDインポーターのサンプル実装を提供します。
//!
//! このサンプルは開発者がプラグインを作成する際の参考になるように
//! 設計されています。

use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::image::{Image, ImageBuffer, Layer, Rgba};
use crate::plugin::types::{
    Host, ImportError, ImportOptions, ImporterPlugin, Plugin, PluginCapabilities, PluginMetadata,
    PluginType, ProjectAccess, SettingsAccess,
};

/// PSDインポータープラグイン
///
/// Adobe Photoshopファイル形式（.psd）をインポートするためのプラグイン。
/// レイヤー構造、テキスト、効果などの要素を保持しながらインポートします。
pub struct PsdImporterPlugin {
    /// プラグインのメタデータ
    metadata: PluginMetadata,
    /// プラグインの機能
    capabilities: PluginCapabilities,
    /// プラグインの設定
    settings: Mutex<HashMap<String, String>>,
    /// ホストへの参照
    host: Option<Arc<dyn Host>>,
    /// プラグインの初期化状態
    initialized: bool,
}

impl PsdImporterPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        // メタデータを設定
        let metadata = PluginMetadata {
            id: "com.edv.psd-importer".to_string(),
            name: "PSD Importer".to_string(),
            version: "1.0.0".to_string(),
            author: "EDV Team".to_string(),
            description: "Adobe Photoshopファイル（.psd）をインポートするプラグイン".to_string(),
            plugin_type: PluginType::Importer,
            api_version: "1.0".to_string(),
        };

        // 機能を設定
        let capabilities = PluginCapabilities {
            has_settings_ui: true,
            supports_hot_reload: true,
            supports_async: true,
            gpu_accelerated: false,
            thread_safe: true,
        };

        // デフォルト設定
        let mut settings = HashMap::new();
        settings.insert("preserve_text_layers".to_string(), "true".to_string());
        settings.insert("import_smart_objects".to_string(), "true".to_string());
        settings.insert("flatten_groups".to_string(), "false".to_string());
        settings.insert("import_hidden_layers".to_string(), "true".to_string());

        Self {
            metadata,
            capabilities,
            settings: Mutex::new(settings),
            host: None,
            initialized: false,
        }
    }

    /// 設定値を取得
    fn get_setting(&self, key: &str, default: &str) -> String {
        let settings = self.settings.lock().unwrap();
        settings
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// 設定値を真偽値として取得
    fn get_setting_as_bool(&self, key: &str, default: bool) -> bool {
        let value = self.get_setting(key, &default.to_string());
        value.parse::<bool>().unwrap_or(default)
    }

    /// PSDファイルのヘッダーを検証
    fn validate_psd_header(&self, data: &[u8]) -> bool {
        // PSDファイルは "8BPS" の4バイトシグネチャで始まる
        if data.len() < 4 {
            return false;
        }

        &data[0..4] == b"8BPS"
    }

    /// レイヤー情報を抽出
    fn extract_layers(
        &self,
        psd_data: &[u8],
        options: &ImportOptions,
    ) -> Result<Vec<Layer>, ImportError> {
        // 実際の実装はより複雑になりますが、例としてシンプルな処理を記載
        // ここではPSDパース用のライブラリを使用することを想定しています

        let preserve_text = self.get_setting_as_bool("preserve_text_layers", true);
        let import_hidden = self.get_setting_as_bool("import_hidden_layers", true);
        let import_smart_objects = self.get_setting_as_bool("import_smart_objects", true);
        let flatten_groups = self.get_setting_as_bool("flatten_groups", false);

        info!(
            "PSDレイヤーを抽出中... 設定: テキストレイヤー保持={preserve_text}, 非表示レイヤー含む={import_hidden}"
        );

        // 実際の実装では、PSDライブラリを使用してレイヤーを抽出します
        // ここではサンプルとして空のレイヤーリストを返します
        let layers = Vec::new();

        debug!("{}レイヤーを抽出しました", layers.len());

        Ok(layers)
    }

    /// メタデータを抽出
    fn extract_metadata(&self, psd_data: &[u8]) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        // 実際の実装では、PSDファイルからメタデータを抽出します
        // ここではサンプルとして空のメタデータを返します
        metadata.insert("format".to_string(), "PSD".to_string());
        metadata.insert("version".to_string(), "1.0".to_string());

        metadata
    }
}

impl Plugin for PsdImporterPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> &PluginCapabilities {
        &self.capabilities
    }

    fn initialize(&mut self, host: Arc<dyn Host>) -> Result<(), String> {
        info!("🖌️ PSDインポータープラグインを初期化中...");
        self.host = Some(host.clone());

        // ホストから保存された設定を読み込む
        if let Some(saved_settings) = host.load_plugin_settings(&self.metadata.id) {
            let mut settings = self.settings.lock().unwrap();
            for (key, value) in saved_settings {
                settings.insert(key, value);
            }
        }

        self.initialized = true;
        info!("✓ PSDインポータープラグインの初期化が完了しました");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        info!("💤 PSDインポータープラグインをシャットダウンします...");

        // 設定を保存
        if let Some(host) = &self.host {
            let settings = self.settings.lock().unwrap();
            host.save_plugin_settings(&self.metadata.id, settings.clone());
        }

        self.initialized = false;
        info!("👋 PSDインポータープラグインのシャットダウンが完了しました");
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl ImporterPlugin for PsdImporterPlugin {
    fn get_supported_extensions(&self) -> Vec<String> {
        vec!["psd".to_string()]
    }

    fn get_importer_name(&self) -> String {
        "Photoshopドキュメントインポーター".to_string()
    }

    fn get_importer_description(&self) -> String {
        "Adobe Photoshop (.psd) ファイルをレイヤーサポート付きでインポートします".to_string()
    }

    fn can_import_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            return ext.to_string_lossy().to_lowercase() == "psd";
        }
        false
    }

    fn import(&self, file_path: &Path, options: &ImportOptions) -> Result<Image, ImportError> {
        if !self.is_initialized() {
            return Err(ImportError::NotInitialized);
        }

        info!("📥 PSDファイルをインポート中: {}", file_path.display());

        // ファイルを読み込む
        let file_data = match std::fs::read(file_path) {
            Ok(data) => data,
            Err(e) => {
                error!("ファイル読み込みエラー: {e}");
                return Err(ImportError::IoError(format!("ファイル読み込みエラー: {e}")));
            }
        };

        // PSDヘッダーの検証
        if !self.validate_psd_header(&file_data) {
            warn!("無効なPSDファイル形式: {}", file_path.display());
            return Err(ImportError::InvalidFormat(
                "有効なPSDファイルではありません".to_string(),
            ));
        }

        // PSDデータを解析する
        let layers = match self.extract_layers(&file_data, options) {
            Ok(layers) => layers,
            Err(e) => {
                error!("レイヤー抽出エラー: {e:?}");
                return Err(e);
            }
        };

        // メタデータを抽出
        let metadata = self.extract_metadata(&file_data);

        // 画像の幅と高さを決定（実際の実装ではPSDファイルから抽出）
        let width = options.width.unwrap_or(800);
        let height = options.height.unwrap_or(600);

        // EDV画像オブジェクトを作成
        let image = Image::new(width, height);

        info!(
            "✓ PSDファイルのインポートが完了しました: {}x{}, {}レイヤー",
            width,
            height,
            layers.len()
        );

        Ok(image)
    }

    fn get_import_progress(&self) -> f32 {
        // 現在のインポート進捗を返す（0.0〜1.0）
        // 非同期インポートの場合に使用
        0.0 // ダミー実装
    }
}

impl SettingsAccess for PsdImporterPlugin {
    fn get_settings(&self) -> HashMap<String, String> {
        self.settings.lock().unwrap().clone()
    }

    fn update_settings(&self, new_settings: HashMap<String, String>) -> Result<(), String> {
        let mut settings = self.settings.lock().unwrap();

        // 設定を更新
        for (key, value) in new_settings {
            settings.insert(key, value);
        }

        // 設定を保存
        if let Some(host) = &self.host {
            host.save_plugin_settings(&self.metadata.id, settings.clone());
        }

        Ok(())
    }
}

// プラグインファクトリ関数
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(PsdImporterPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let plugin = PsdImporterPlugin::new();
        assert_eq!(plugin.metadata().name, "PSD Importer");
        assert_eq!(plugin.metadata().plugin_type, PluginType::Importer);
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = PsdImporterPlugin::new();
        assert!(plugin.capabilities().supports_async);
        assert!(!plugin.capabilities().gpu_accelerated);
    }

    #[test]
    fn test_supported_extensions() {
        let plugin = PsdImporterPlugin::new();
        let extensions = plugin.get_supported_extensions();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0], "psd");
    }

    #[test]
    fn test_settings_access() {
        let plugin = PsdImporterPlugin::new();
        let settings = plugin.get_settings();

        assert_eq!(
            settings.get("preserve_text_layers"),
            Some(&"true".to_string())
        );
        assert_eq!(
            settings.get("import_smart_objects"),
            Some(&"true".to_string())
        );
    }
}
