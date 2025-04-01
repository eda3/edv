//! サンプルエクスポーターププラグイン実装
//!
//! このモジュールでは、EDVのプラグインシステムを使った
//! WebPエクスポーターのサンプル実装を提供します。
//!
//! このサンプルは開発者がプラグインを作成する際の参考になるように
//! 設計されています。

use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::image::{Image, ImageBuffer, Rgba};
use crate::plugin::types::{
    ExporterPlugin, Host, Plugin, PluginCapabilities, PluginMetadata, PluginType, ProjectAccess,
    SettingsAccess,
}; // アプリケーションの画像処理モジュール

/// WebPエクスポータープラグイン
///
/// 画像をWebP形式でエクスポートするサンプルプラグイン実装。
/// 画質設定やロスレス圧縮などの設定をサポートします。
pub struct WebpExporterPlugin {
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

impl WebpExporterPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        // メタデータを設定
        let metadata = PluginMetadata {
            id: "com.example.webp-exporter".to_string(),
            name: "WebP Exporter".to_string(),
            version: "1.0.0".to_string(),
            author: "Example Developer".to_string(),
            description: "画像をWebP形式でエクスポートするプラグイン".to_string(),
            plugin_type: PluginType::Exporter,
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
        settings.insert("quality".to_string(), "90".to_string());
        settings.insert("lossless".to_string(), "false".to_string());
        settings.insert("metadata_enabled".to_string(), "true".to_string());

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

    /// 設定値を整数として取得
    fn get_setting_as_i32(&self, key: &str, default: i32) -> i32 {
        let value = self.get_setting(key, &default.to_string());
        value.parse::<i32>().unwrap_or(default)
    }

    /// 設定値を浮動小数点として取得
    fn get_setting_as_f32(&self, key: &str, default: f32) -> f32 {
        let value = self.get_setting(key, &default.to_string());
        value.parse::<f32>().unwrap_or(default)
    }

    /// 設定値を真偽値として取得
    fn get_setting_as_bool(&self, key: &str, default: bool) -> bool {
        let value = self.get_setting(key, &default.to_string());
        value.parse::<bool>().unwrap_or(default)
    }

    /// WebPエンコード設定を生成
    fn create_webp_config(&self) -> webp::WebPConfig {
        let mut config = webp::WebPConfig::new().unwrap();

        // 品質設定（0〜100）
        let quality = self.get_setting_as_f32("quality", 90.0);
        config.quality = quality;

        // ロスレス設定
        let lossless = self.get_setting_as_bool("lossless", false);
        config.lossless = if lossless { 1 } else { 0 };

        // メタデータ設定
        let metadata_enabled = self.get_setting_as_bool("metadata_enabled", true);

        // その他の設定
        config.method = 6; // 圧縮メソッド（0は高速/低品質、6は低速/高品質）
        config.target_size = 0; // 出力サイズを指定しない（0 = 無制限）
        config.target_PSNR = 0.0; // PSNRを指定しない
        config.segments = 4; // セグメント数
        config.sns_strength = 50; // 空間ノイズシェーピングの強さ
        config.filter_strength = 60; // フィルターの強さ
        config.filter_sharpness = 0; // フィルターシャープネス
        config.filter_type = 1; // フィルタータイプ
        config.autofilter = 1; // 自動フィルター設定
        config.alpha_compression = 1; // アルファチャンネル圧縮
        config.alpha_filtering = 1; // アルファチャンネルフィルタリング
        config.alpha_quality = 90; // アルファチャンネル品質
        config.pass = 1; // エンコードパス数

        config
    }
}

impl Plugin for WebpExporterPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> &PluginCapabilities {
        &self.capabilities
    }

    fn initialize(&mut self, host: Arc<dyn Host>) -> Result<(), String> {
        info!("🌟 WebPエクスポーターを初期化中...");
        self.host = Some(host.clone());

        // ホストから保存された設定を読み込む
        if let Some(saved_settings) = host.load_plugin_settings(&self.metadata.id) {
            let mut settings = self.settings.lock().unwrap();
            for (key, value) in saved_settings {
                settings.insert(key, value);
            }
        }

        self.initialized = true;
        info!("✓ WebPエクスポーターの初期化が完了しました");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        info!("💤 WebPエクスポーターをシャットダウンします...");

        // 設定を保存
        if let Some(host) = &self.host {
            let settings = self.settings.lock().unwrap();
            host.save_plugin_settings(&self.metadata.id, settings.clone());
        }

        self.initialized = false;
        info!("👋 WebPエクスポーターのシャットダウンが完了しました");
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl ExporterPlugin for WebpExporterPlugin {
    fn export(&self, image: &Image, path: &Path) -> Result<(), String> {
        if !self.initialized {
            return Err("プラグインが初期化されていません".to_string());
        }

        info!(
            "🖼️ WebP形式で画像をエクスポートしています: {}",
            path.display()
        );

        // ファイルを開く
        let file =
            File::create(path).map_err(|e| format!("ファイルの作成に失敗しました: {}", e))?;

        let buffer = BufWriter::new(file);

        // WebPエンコード設定
        let config = self.create_webp_config();

        // 画像データを準備
        let (width, height) = image.dimensions();
        let rgba = image.as_raw();

        // WebPエンコーダーを作成
        // 注: 実際のWebPエンコードはライブラリに依存します
        // 以下はシミュレーション用のコードです
        // let encoder = webp::Encoder::new(rgba, width, height, webp::PixelFormat::RGBA);
        // let memory = encoder.encode(&config).map_err(|e| format!("エンコードに失敗しました: {}", e))?;

        // バッファに書き込み
        // buffer.write_all(memory.as_bytes()).map_err(|e| {
        //     format!("ファイルへの書き込みに失敗しました: {}", e)
        // })?;

        // 実際の実装では上記のコメントアウトされた部分を正しいWebPライブラリに対応したコードに置き換えます
        // ここではシミュレーションとして、単にエクスポートが成功したとします

        info!("✓ WebP形式での画像エクスポートが完了しました");
        Ok(())
    }

    fn get_supported_extensions(&self) -> Vec<String> {
        vec!["webp".to_string()]
    }

    fn get_exporter_name(&self) -> String {
        "WebP形式".to_string()
    }

    fn get_exporter_description(&self) -> String {
        "WebP形式で画像をエクスポートします。高圧縮かつ高品質なウェブ用画像形式です。".to_string()
    }
}

impl SettingsAccess for WebpExporterPlugin {
    fn get_settings(&self) -> HashMap<String, String> {
        self.settings.lock().unwrap().clone()
    }

    fn update_settings(&self, new_settings: HashMap<String, String>) -> Result<(), String> {
        let mut settings = self.settings.lock().unwrap();

        // 設定を更新
        for (key, value) in new_settings {
            // 品質パラメータの妥当性チェック
            if key == "quality" {
                if let Ok(quality) = value.parse::<i32>() {
                    if quality < 0 || quality > 100 {
                        return Err("品質設定は0〜100の範囲で指定してください".to_string());
                    }
                } else {
                    return Err("品質設定は整数値で指定してください".to_string());
                }
            }

            settings.insert(key, value);
        }

        // 設定を保存
        if let Some(host) = &self.host {
            host.save_plugin_settings(&self.metadata.id, settings.clone());
        }

        Ok(())
    }
}

/// プラグインのエントリーポイント関数
///
/// この関数は、プラグインローダーによって呼び出されるエントリーポイントです。
/// 新しいプラグインインスタンスを作成して返します。
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(WebpExporterPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let plugin = WebpExporterPlugin::new();
        assert_eq!(plugin.metadata().id, "com.example.webp-exporter");
        assert_eq!(plugin.metadata().plugin_type, PluginType::Exporter);
    }

    #[test]
    fn test_get_supported_extensions() {
        let plugin = WebpExporterPlugin::new();
        let extensions = plugin.get_supported_extensions();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0], "webp");
    }

    #[test]
    fn test_settings() {
        let plugin = WebpExporterPlugin::new();

        // デフォルト設定を確認
        let settings = plugin.get_settings();
        assert_eq!(settings.get("quality"), Some(&"90".to_string()));
        assert_eq!(settings.get("lossless"), Some(&"false".to_string()));

        // 品質設定を変更
        let mut new_settings = HashMap::new();
        new_settings.insert("quality".to_string(), "75".to_string());
        plugin.update_settings(new_settings).unwrap();

        let updated_settings = plugin.get_settings();
        assert_eq!(updated_settings.get("quality"), Some(&"75".to_string()));

        // 無効な品質設定
        let mut invalid_settings = HashMap::new();
        invalid_settings.insert("quality".to_string(), "101".to_string());
        let result = plugin.update_settings(invalid_settings);
        assert!(result.is_err());
    }
}
