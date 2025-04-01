//! プラグインシステムモジュール
//!
//! このモジュールはEDVアプリケーションのプラグインシステムを実装します。
//! プラグインのライフサイクル管理（読み込み、初期化、実行、シャットダウン、アンロード）
//! などの機能を提供します。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

// 公開モジュール
pub mod examples;
pub mod host;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod registry;
pub mod security;
pub mod types;

// エラー型
/// プラグイン操作に関連するエラー
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// プラグインが見つかりません
    #[error("プラグインが見つかりません: {0}")]
    PluginNotFound(String),

    /// プラグインの読み込みに失敗しました
    #[error("プラグインの読み込みに失敗しました: {0}")]
    LoadingFailed(String),

    /// プラグインの初期化に失敗しました
    #[error("プラグインの初期化に失敗しました: {0}")]
    InitializationFailed(String),

    /// プラグインのシャットダウンに失敗しました
    #[error("プラグインのシャットダウンに失敗しました: {0}")]
    ShutdownFailed(String),

    /// プラグインの実行に失敗しました
    #[error("プラグインの実行に失敗しました: {0}")]
    ExecutionFailed(String),

    /// プラグインの依存関係に問題があります
    #[error("プラグインの依存関係エラー: {0}")]
    DependencyError(String),

    /// プラグインのアクセス権限がありません
    #[error("プラグインにアクセス権限がありません: {0}")]
    PermissionDenied(String),

    /// プラグインAPIのバージョンが互換性がありません
    #[error("プラグインAPIのバージョンが互換性がありません: {0}")]
    IncompatibleApiVersion(String),

    /// 一般的なプラグインエラー
    #[error("プラグインエラー: {0}")]
    GenericError(String),
}

/// プラグイン操作の結果型
pub type PluginResult<T> = Result<T, PluginError>;

/// プラグインシステムを初期化し、プラグインマネージャを返す
pub fn init() -> PluginResult<Arc<manager::PluginManager>> {
    let options = manager::PluginInitOptions {
        auto_load: true,
        auto_initialize_trusted: true,
        additional_search_paths: Vec::new(),
    };

    let manager = manager::PluginManager::new(options)?;
    Ok(Arc::new(manager))
}

/// システムプラグインディレクトリのパスを取得
pub fn get_system_plugin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // OSに基づいてシステムプラグインディレクトリを追加
    if cfg!(target_os = "linux") {
        // Linuxのシステムプラグインディレクトリ
        dirs.push(PathBuf::from("/usr/lib/edv/plugins"));
        dirs.push(PathBuf::from("/usr/local/lib/edv/plugins"));
    } else if cfg!(target_os = "macos") {
        // macOSのシステムプラグインディレクトリ
        dirs.push(PathBuf::from("/Library/Application Support/EDV/Plugins"));
        dirs.push(PathBuf::from("/usr/local/lib/edv/plugins"));
    } else if cfg!(target_os = "windows") {
        // Windowsのシステムプラグインディレクトリ
        dirs.push(PathBuf::from(r"C:\Program Files\EDV\Plugins"));
        dirs.push(PathBuf::from(r"C:\Program Files (x86)\EDV\Plugins"));
    }

    dirs
}

/// ユーザープラグインディレクトリのパスを取得
pub fn get_user_plugin_dir() -> PathBuf {
    let mut user_plugin_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    user_plugin_dir.push("EDV");
    user_plugin_dir.push("plugins");

    // ディレクトリが存在しない場合は作成
    if !user_plugin_dir.exists() {
        std::fs::create_dir_all(&user_plugin_dir).unwrap_or_else(|_| {
            eprintln!("警告: ユーザープラグインディレクトリの作成に失敗しました");
        });
    }

    user_plugin_dir
}

/// プラグインの状態を文字列に変換
pub fn plugin_state_to_string(state: types::PluginState) -> &'static str {
    match state {
        types::PluginState::Discovered => "発見済み",
        types::PluginState::Loaded => "読み込み済み",
        types::PluginState::Active => "アクティブ",
        types::PluginState::Paused => "一時停止",
        types::PluginState::Inactive => "非アクティブ",
        types::PluginState::Error => "エラー",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plugin_dirs() {
        // システムプラグインディレクトリの取得をテスト
        let system_dirs = get_system_plugin_dirs();
        assert!(!system_dirs.is_empty());

        // ユーザープラグインディレクトリの取得をテスト
        let user_dir = get_user_plugin_dir();
        assert!(!user_dir.to_string_lossy().is_empty());
    }

    #[test]
    fn test_plugin_state_to_string() {
        assert_eq!(
            plugin_state_to_string(types::PluginState::Discovered),
            "発見済み"
        );
        assert_eq!(
            plugin_state_to_string(types::PluginState::Loaded),
            "読み込み済み"
        );
        assert_eq!(
            plugin_state_to_string(types::PluginState::Active),
            "アクティブ"
        );
        assert_eq!(
            plugin_state_to_string(types::PluginState::Paused),
            "一時停止"
        );
        assert_eq!(
            plugin_state_to_string(types::PluginState::Inactive),
            "非アクティブ"
        );
        assert_eq!(plugin_state_to_string(types::PluginState::Error), "エラー");
    }
}
