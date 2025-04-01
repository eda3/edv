//! サンプルエフェクトプラグイン実装
//!
//! このモジュールでは、EDVのプラグインシステムを使った
//! かわいいエフェクトプラグインのサンプル実装を提供します。
//!
//! このサンプルは開発者がプラグインを作成する際の参考になるように
//! 設計されています。

use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::image::{Image, ImageBuffer, Rgba};
use crate::plugin::types::{
    EffectPlugin, Host, Plugin, PluginCapabilities, PluginMetadata, PluginType, ProjectAccess,
    SettingsAccess,
}; // アプリケーションの画像処理モジュール

/// かわいいエフェクトプラグイン
///
/// 画像にかわいいエフェクト（ピンク色の強調、パステル調整、ハートのオーバーレイなど）を
/// 適用するプラグインのサンプル実装
pub struct KawaiiEffectPlugin {
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

impl KawaiiEffectPlugin {
    /// 新しいプラグインインスタンスを作成
    pub fn new() -> Self {
        // メタデータを設定
        let metadata = PluginMetadata {
            id: "com.example.kawaii-effect".to_string(),
            name: "Kawaii Effect".to_string(),
            version: "1.0.0".to_string(),
            author: "Example Developer".to_string(),
            description: "画像にかわいい効果を適用するプラグイン".to_string(),
            plugin_type: PluginType::Effect,
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
        settings.insert("pink_intensity".to_string(), "0.5".to_string());
        settings.insert("add_hearts".to_string(), "true".to_string());
        settings.insert("pastel_level".to_string(), "0.7".to_string());

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

    /// ピンク色を強調するエフェクト
    fn enhance_pink(&self, image: &mut Image) {
        let pink_intensity = self.get_setting_as_f32("pink_intensity", 0.5);

        // 画像の各ピクセルを処理
        for pixel in image.pixels_mut() {
            // ピンク色に近い色を強調
            if pixel[0] > pixel[2] && pixel[0] > 100 {
                pixel[0] = ((pixel[0] as f32) * (1.0 + pink_intensity)).min(255.0) as u8;
                pixel[1] = ((pixel[1] as f32) * (0.9)).max(0.0) as u8;
            }
        }
    }

    /// パステル調整エフェクト
    fn apply_pastel(&self, image: &mut Image) {
        let pastel_level = self.get_setting_as_f32("pastel_level", 0.7);

        // 画像の各ピクセルを処理
        for pixel in image.pixels_mut() {
            // 明るさを計算
            let brightness =
                0.299 * (pixel[0] as f32) + 0.587 * (pixel[1] as f32) + 0.114 * (pixel[2] as f32);

            // パステル効果を適用（色を明るい方向に、彩度を下げる方向に）
            pixel[0] = ((pixel[0] as f32) * (1.0 - pastel_level)
                + (brightness + 50.0) * pastel_level)
                .min(255.0) as u8;
            pixel[1] = ((pixel[1] as f32) * (1.0 - pastel_level)
                + (brightness + 50.0) * pastel_level)
                .min(255.0) as u8;
            pixel[2] = ((pixel[2] as f32) * (1.0 - pastel_level)
                + (brightness + 50.0) * pastel_level)
                .min(255.0) as u8;
        }
    }

    /// ハートのオーバーレイを追加
    fn add_heart_overlay(&self, image: &mut Image) {
        if !self.get_setting_as_bool("add_hearts", true) {
            return;
        }

        // 画像のサイズを取得
        let (width, height) = image.dimensions();

        // ハートの位置とサイズを決定
        let heart_size = width.min(height) as i32 / 10;
        let positions = [
            (width as i32 / 4, height as i32 / 4),
            (width as i32 * 3 / 4, height as i32 / 4),
            (width as i32 / 2, height as i32 * 3 / 4),
        ];

        for &(center_x, center_y) in &positions {
            self.draw_heart(image, center_x, center_y, heart_size);
        }
    }

    /// ハートを描画する
    fn draw_heart(&self, image: &mut Image, center_x: i32, center_y: i32, size: i32) {
        // ハート形状の方程式に基づいて描画
        for dy in -size..size {
            for dx in -size..size {
                // ハート形状の方程式: (x^2 + y^2 - 1)^3 - x^2*y^3 <= 0 を変形
                let x = dx as f32 / size as f32;
                let y = -dy as f32 / size as f32;

                // ハートの形状内部なら描画
                if (x * x + y * y - 1.0).powi(3) - x * x * y.powi(3) <= 0.0 {
                    let px = center_x + dx;
                    let py = center_y + dy;

                    // 画像の範囲内かチェック
                    if px >= 0 && px < image.width() as i32 && py >= 0 && py < image.height() as i32
                    {
                        // ピンク色のハートを描画（半透明）
                        let mut pixel = image.get_pixel(px as u32, py as u32);
                        pixel[0] = ((pixel[0] as f32) * 0.7 + 255.0 * 0.3).min(255.0) as u8; // R
                        pixel[1] = ((pixel[1] as f32) * 0.7 + 150.0 * 0.3).min(255.0) as u8; // G
                        pixel[2] = ((pixel[2] as f32) * 0.7 + 220.0 * 0.3).min(255.0) as u8; // B
                        image.put_pixel(px as u32, py as u32, pixel);
                    }
                }
            }
        }
    }
}

impl Plugin for KawaiiEffectPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> &PluginCapabilities {
        &self.capabilities
    }

    fn initialize(&mut self, host: Arc<dyn Host>) -> Result<(), String> {
        info!("🌸 かわいいエフェクトプラグインを初期化中...");
        self.host = Some(host.clone());

        // ホストから保存された設定を読み込む
        if let Some(saved_settings) = host.load_plugin_settings(&self.metadata.id) {
            let mut settings = self.settings.lock().unwrap();
            for (key, value) in saved_settings {
                settings.insert(key, value);
            }
        }

        self.initialized = true;
        info!("✨ かわいいエフェクトプラグインの初期化が完了しました！");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        info!("😢 かわいいエフェクトプラグインをシャットダウンします...");

        // 設定を保存
        if let Some(host) = &self.host {
            let settings = self.settings.lock().unwrap();
            host.save_plugin_settings(&self.metadata.id, settings.clone());
        }

        self.initialized = false;
        info!("👋 かわいいエフェクトプラグインのシャットダウンが完了しました");
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl EffectPlugin for KawaiiEffectPlugin {
    fn apply_effect(&self, image: &mut Image) -> Result<(), String> {
        if !self.initialized {
            return Err("プラグインが初期化されていません".to_string());
        }

        info!("🎀 かわいいエフェクトを適用しています...");

        // エフェクトの適用順序
        self.enhance_pink(image);
        self.apply_pastel(image);
        self.add_heart_overlay(image);

        info!("✨ かわいいエフェクトの適用が完了しました！");
        Ok(())
    }

    fn get_effect_name(&self) -> String {
        "Kawaii Effect".to_string()
    }

    fn get_effect_description(&self) -> String {
        "画像にかわいいピンク調のエフェクトとハートを追加します".to_string()
    }
}

impl SettingsAccess for KawaiiEffectPlugin {
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

/// プラグインのエントリーポイント関数
///
/// この関数は、プラグインローダーによって呼び出されるエントリーポイントです。
/// 新しいプラグインインスタンスを作成して返します。
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(KawaiiEffectPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let plugin = KawaiiEffectPlugin::new();
        assert_eq!(plugin.metadata().id, "com.example.kawaii-effect");
        assert_eq!(plugin.metadata().plugin_type, PluginType::Effect);
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = KawaiiEffectPlugin::new();
        assert!(plugin.capabilities().has_settings_ui);
        assert!(plugin.capabilities().supports_hot_reload);
        assert!(plugin.capabilities().thread_safe);
    }

    #[test]
    fn test_settings_access() {
        let plugin = KawaiiEffectPlugin::new();
        let settings = plugin.get_settings();

        assert_eq!(settings.get("pink_intensity"), Some(&"0.5".to_string()));
        assert_eq!(settings.get("add_hearts"), Some(&"true".to_string()));

        // 設定の更新をテスト
        let mut new_settings = HashMap::new();
        new_settings.insert("pink_intensity".to_string(), "0.8".to_string());
        plugin.update_settings(new_settings).unwrap();

        let updated_settings = plugin.get_settings();
        assert_eq!(
            updated_settings.get("pink_intensity"),
            Some(&"0.8".to_string())
        );
        assert_eq!(
            updated_settings.get("add_hearts"),
            Some(&"true".to_string())
        );
    }
}
