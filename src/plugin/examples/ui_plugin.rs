//! サンプルUIプラグイン実装
//!
//! このモジュールでは、EDVのプラグインシステムを使った
//! カスタムパネルUIプラグインのサンプル実装を提供します。
//!
//! このサンプルは開発者がプラグインを作成する際の参考になるように
//! 設計されています。

use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::plugin::types::{
    Host, Plugin, PluginCapabilities, PluginMetadata, PluginType, SettingsAccess,
    UiComponentRegistry, UiComponentType, UiPlugin,
};

/// カスタムパネルUIプラグイン
///
/// EDVに新しいパネル、テーマ、ツールなどの機能を追加するためのプラグイン。
/// ユーザーワークフローをカスタマイズし、生産性を向上させることができます。
pub struct CustomPanelUiPlugin {
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

impl CustomPanelUiPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        // メタデータを設定
        let metadata = PluginMetadata {
            id: "com.edv.custom-panel-ui".to_string(),
            name: "Custom Panel UI".to_string(),
            version: "1.0.0".to_string(),
            author: "EDV Team".to_string(),
            description: "EDVにカスタムパネルとテーマを追加するプラグイン".to_string(),
            plugin_type: PluginType::Ui,
            api_version: "1.0".to_string(),
        };

        // 機能を設定
        let capabilities = PluginCapabilities {
            has_settings_ui: true,
            supports_hot_reload: true,
            supports_async: false,
            gpu_accelerated: false,
            thread_safe: true,
        };

        // デフォルト設定
        let mut settings = HashMap::new();
        settings.insert("theme".to_string(), "dark".to_string());
        settings.insert("show_custom_tools".to_string(), "true".to_string());
        settings.insert("show_extended_properties".to_string(), "true".to_string());
        settings.insert("enable_shortcuts".to_string(), "true".to_string());
        settings.insert("language".to_string(), "ja".to_string());

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

    /// 現在のテーマに基づいて色設定を取得
    fn get_theme_colors(&self) -> HashMap<String, String> {
        let theme = self.get_setting("theme", "dark");
        let mut colors = HashMap::new();

        match theme.as_str() {
            "dark" => {
                colors.insert("background".to_string(), "#1e1e1e".to_string());
                colors.insert("foreground".to_string(), "#d4d4d4".to_string());
                colors.insert("accent".to_string(), "#0078d7".to_string());
                colors.insert("panel".to_string(), "#252526".to_string());
                colors.insert("border".to_string(), "#3f3f3f".to_string());
            }
            "light" => {
                colors.insert("background".to_string(), "#f5f5f5".to_string());
                colors.insert("foreground".to_string(), "#333333".to_string());
                colors.insert("accent".to_string(), "#0078d7".to_string());
                colors.insert("panel".to_string(), "#ffffff".to_string());
                colors.insert("border".to_string(), "#d0d0d0".to_string());
            }
            "pastel" => {
                colors.insert("background".to_string(), "#f0e6f6".to_string());
                colors.insert("foreground".to_string(), "#5a5a5a".to_string());
                colors.insert("accent".to_string(), "#b39ddb".to_string());
                colors.insert("panel".to_string(), "#fff1e6".to_string());
                colors.insert("border".to_string(), "#d8c8e7".to_string());
            }
            _ => {
                // デフォルトはダークテーマ
                colors.insert("background".to_string(), "#1e1e1e".to_string());
                colors.insert("foreground".to_string(), "#d4d4d4".to_string());
                colors.insert("accent".to_string(), "#0078d7".to_string());
                colors.insert("panel".to_string(), "#252526".to_string());
                colors.insert("border".to_string(), "#3f3f3f".to_string());
            }
        }

        colors
    }

    /// ショートカットキー設定を取得
    fn get_keyboard_shortcuts(&self) -> Vec<(String, String)> {
        let mut shortcuts = Vec::new();

        shortcuts.push((
            "custom_panel_toggle".to_string(),
            "Ctrl+Shift+P".to_string(),
        ));
        shortcuts.push(("color_picker".to_string(), "Alt+C".to_string()));
        shortcuts.push(("quick_export".to_string(), "Ctrl+Alt+E".to_string()));

        shortcuts
    }
}

impl Plugin for CustomPanelUiPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> &PluginCapabilities {
        &self.capabilities
    }

    fn initialize(&mut self, host: Arc<dyn Host>) -> Result<(), String> {
        info!("💫 カスタムパネルUIプラグインを初期化中...");
        self.host = Some(host.clone());

        // ホストから保存された設定を読み込む
        if let Some(saved_settings) = host.load_plugin_settings(&self.metadata.id) {
            let mut settings = self.settings.lock().unwrap();
            for (key, value) in saved_settings {
                settings.insert(key, value);
            }
        }

        // 現在のテーマを適用
        let theme = self.get_setting("theme", "dark");
        info!("テーマを適用中: {}", theme);

        self.initialized = true;
        info!("✓ カスタムパネルUIプラグインの初期化が完了しました");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        info!("🌙 カスタムパネルUIプラグインをシャットダウンします...");

        // 設定を保存
        if let Some(host) = &self.host {
            let settings = self.settings.lock().unwrap();
            host.save_plugin_settings(&self.metadata.id, settings.clone());
        }

        self.initialized = false;
        info!("👋 カスタムパネルUIプラグインのシャットダウンが完了しました");
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl UiPlugin for CustomPanelUiPlugin {
    fn register_components(&self, registry: &mut dyn UiComponentRegistry) -> Result<(), String> {
        if !self.is_initialized() {
            return Err("プラグインが初期化されていません".to_string());
        }

        info!("🧩 UIコンポーネントを登録中...");

        let show_custom_tools = self.get_setting_as_bool("show_custom_tools", true);
        let show_extended_properties = self.get_setting_as_bool("show_extended_properties", true);

        // カスタムツールパネルを登録
        if show_custom_tools {
            debug!("カスタムツールパネルを登録中");
            registry.register_component(
                "custom_tools_panel",
                UiComponentType::Panel,
                "カスタムツール",
                "left",
                2,
            )?;
        }

        // 拡張プロパティパネルを登録
        if show_extended_properties {
            debug!("拡張プロパティパネルを登録中");
            registry.register_component(
                "extended_properties_panel",
                UiComponentType::Panel,
                "拡張プロパティ",
                "right",
                1,
            )?;
        }

        // カスタムメニュー項目を登録
        registry.register_component(
            "custom_menu",
            UiComponentType::MenuItem,
            "カスタム機能",
            "tools",
            0,
        )?;

        // キーボードショートカットを登録
        if self.get_setting_as_bool("enable_shortcuts", true) {
            debug!("キーボードショートカットを登録中");
            let shortcuts = self.get_keyboard_shortcuts();
            for (action, key) in shortcuts {
                registry.register_shortcut(&action, &key)?;
            }
        }

        info!("✓ UIコンポーネントの登録が完了しました");
        Ok(())
    }

    fn get_theme_data(&self) -> HashMap<String, String> {
        // 現在のテーマに基づく色設定を返す
        self.get_theme_colors()
    }

    fn handle_component_action(
        &self,
        component_id: &str,
        action: &str,
        data: &str,
    ) -> Result<String, String> {
        info!(
            "コンポーネントアクション: {} -> {} (データ: {})",
            component_id, action, data
        );

        match (component_id, action) {
            ("custom_tools_panel", "activate") => {
                // カスタムツールがアクティブになったときの処理
                Ok("カスタムツールがアクティブになりました".to_string())
            }

            ("extended_properties_panel", "update") => {
                // プロパティが更新されたときの処理
                Ok(format!("プロパティを更新しました: {}", data))
            }

            ("custom_menu", "click") => {
                // メニュー項目がクリックされたときの処理
                Ok("メニューアクションを実行しました".to_string())
            }

            _ => {
                warn!(
                    "未知のコンポーネントアクション: {}:{}",
                    component_id, action
                );
                Err(format!(
                    "未知のコンポーネントまたはアクション: {}:{}",
                    component_id, action
                ))
            }
        }
    }

    fn get_component_data(&self, component_id: &str) -> Result<HashMap<String, String>, String> {
        debug!("コンポーネントデータをリクエスト: {}", component_id);

        let mut data = HashMap::new();

        match component_id {
            "custom_tools_panel" => {
                data.insert("title".to_string(), "カスタムツール".to_string());
                data.insert("icon".to_string(), "tools".to_string());
                data.insert("tooltip".to_string(), "カスタム編集ツール".to_string());
            }

            "extended_properties_panel" => {
                data.insert("title".to_string(), "拡張プロパティ".to_string());
                data.insert("icon".to_string(), "settings".to_string());
                data.insert(
                    "tooltip".to_string(),
                    "追加の画像プロパティを表示".to_string(),
                );
                data.insert("width".to_string(), "300".to_string());
            }

            "custom_menu" => {
                data.insert("title".to_string(), "カスタム機能".to_string());
                data.insert("submenu".to_string(), "true".to_string());
            }

            _ => {
                return Err(format!("未知のコンポーネントID: {}", component_id));
            }
        }

        Ok(data)
    }
}

impl SettingsAccess for CustomPanelUiPlugin {
    fn get_settings(&self) -> HashMap<String, String> {
        self.settings.lock().unwrap().clone()
    }

    fn update_settings(&self, new_settings: HashMap<String, String>) -> Result<(), String> {
        let mut settings = self.settings.lock().unwrap();
        let old_theme = settings.get("theme").cloned();

        // 設定を更新
        for (key, value) in new_settings {
            settings.insert(key, value);
        }

        // テーマが変更された場合の処理
        let new_theme = settings.get("theme").cloned();
        if old_theme != new_theme {
            info!("テーマが変更されました: {:?} -> {:?}", old_theme, new_theme);

            // ホストに通知（実際の実装ではUIの更新をトリガー）
            if let Some(host) = &self.host {
                if let Err(e) = host.notify_plugin_event(&self.metadata.id, "theme_changed", "") {
                    warn!("テーマ変更通知エラー: {}", e);
                }
            }
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
    Box::new(CustomPanelUiPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let plugin = CustomPanelUiPlugin::new();
        assert_eq!(plugin.metadata().name, "Custom Panel UI");
        assert_eq!(plugin.metadata().plugin_type, PluginType::Ui);
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = CustomPanelUiPlugin::new();
        assert!(plugin.capabilities().has_settings_ui);
        assert!(plugin.capabilities().supports_hot_reload);
    }

    #[test]
    fn test_theme_colors() {
        let plugin = CustomPanelUiPlugin::new();
        let colors = plugin.get_theme_colors();

        // デフォルトテーマはダーク
        assert!(colors.contains_key("background"));
        assert!(colors.contains_key("foreground"));
        assert!(colors.contains_key("accent"));
    }

    #[test]
    fn test_settings_access() {
        let plugin = CustomPanelUiPlugin::new();
        let settings = plugin.get_settings();

        assert_eq!(settings.get("theme"), Some(&"dark".to_string()));
        assert_eq!(settings.get("show_custom_tools"), Some(&"true".to_string()));
    }
}
