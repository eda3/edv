//! プラグインマニフェスト処理モジュール
//!
//! プラグインマニフェスト（plugin.toml）の読み込み、検証、解析を行うための機能を提供します。
//! マニフェストからプラグインのメタデータを抽出し、依存関係の管理も行います。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml;

use super::PluginError;
use super::types::{PluginCapabilities, PluginMetadata, PluginType};

/// プラグインの依存関係情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// 依存するプラグインのID
    pub id: String,
    /// 必要な最小バージョン (オプション)
    #[serde(rename = "min-version", skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
    /// サポートする最大バージョン (オプション)
    #[serde(rename = "max-version", skip_serializing_if = "Option::is_none")]
    pub max_version: Option<String>,
    /// この依存関係が必須かどうか
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

/// マニフェストファイルの構造
#[derive(Debug, Deserialize)]
struct ManifestFile {
    /// 一般的なプラグイン情報
    plugin: PluginInfo,
    /// 依存関係 (オプション)
    #[serde(default)]
    dependencies: HashMap<String, DependencyInfo>,
    /// 機能 (オプション)
    #[serde(default)]
    capabilities: CapabilitiesInfo,
}

/// プラグイン情報のセクション
#[derive(Debug, Deserialize)]
struct PluginInfo {
    /// プラグインのID
    id: String,
    /// プラグインの名前
    name: String,
    /// プラグインのバージョン
    version: String,
    /// プラグイン作者
    author: String,
    /// プラグインの説明
    description: String,
    /// プラグインの種類
    #[serde(rename = "type")]
    plugin_type: String,
    /// プラグインのAPIバージョン
    api_version: String,
    /// プラグインエントリーポイント関数名
    entry_point: String,
}

/// 依存関係情報のセクション
#[derive(Debug, Deserialize)]
struct DependencyInfo {
    /// 最小バージョン (オプション)
    #[serde(rename = "min-version", default)]
    min_version: Option<String>,
    /// 最大バージョン (オプション)
    #[serde(rename = "max-version", default)]
    max_version: Option<String>,
    /// 必須かどうか
    #[serde(default = "default_required")]
    required: bool,
}

/// 機能情報のセクション
#[derive(Debug, Default, Deserialize)]
struct CapabilitiesInfo {
    /// 設定UIをサポートしているか
    #[serde(default)]
    has_settings_ui: bool,
    /// ホットリロードをサポートしているか
    #[serde(default)]
    supports_hot_reload: bool,
    /// 非同期処理をサポートしているか
    #[serde(default)]
    supports_async: bool,
    /// GPUアクセラレーションをサポートしているか
    #[serde(default)]
    supports_gpu: bool,
    /// スレッドセーフかどうか
    #[serde(default)]
    thread_safe: bool,
}

/// マニフェストの検証結果
#[derive(Debug)]
pub struct ManifestValidationResult {
    /// 検証が成功したかどうか
    pub is_valid: bool,
    /// 警告メッセージリスト
    pub warnings: Vec<String>,
    /// エラーメッセージリスト
    pub errors: Vec<String>,
}

impl ManifestValidationResult {
    /// 新しい検証結果を作成
    fn new() -> Self {
        Self {
            is_valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 警告を追加
    fn add_warning(&mut self, message: String) {
        self.warnings.push(message);
    }

    /// エラーを追加
    fn add_error(&mut self, message: String) {
        self.is_valid = false;
        self.errors.push(message);
    }
}

/// マニフェストファイルを読み込む
pub fn load_manifest(manifest_path: &Path) -> Result<ManifestFile, PluginError> {
    if !manifest_path.exists() {
        return Err(PluginError::ManifestNotFound(
            manifest_path.to_string_lossy().to_string(),
        ));
    }

    let content = fs::read_to_string(manifest_path).map_err(|e| PluginError::IO(e.to_string()))?;

    let manifest: ManifestFile = toml::from_str(&content)
        .map_err(|e| PluginError::InvalidManifest(format!("TOML解析エラー: {}", e)))?;

    Ok(manifest)
}

/// マニフェストからプラグインメタデータを抽出
pub fn extract_metadata(manifest: &ManifestFile) -> Result<PluginMetadata, PluginError> {
    // バージョン文字列を解析
    let version = parse_version(&manifest.plugin.version)?;

    // APIバージョン文字列を解析
    let api_version = parse_api_version(&manifest.plugin.api_version)?;

    // プラグイン種類を解析
    let plugin_type = PluginType::from_str(&manifest.plugin.plugin_type).ok_or_else(|| {
        PluginError::InvalidManifest(format!(
            "不明なプラグイン種類: {}",
            manifest.plugin.plugin_type
        ))
    })?;

    Ok(PluginMetadata {
        id: manifest.plugin.id.clone(),
        name: manifest.plugin.name.clone(),
        version,
        author: manifest.plugin.author.clone(),
        description: manifest.plugin.description.clone(),
        plugin_type,
        api_version,
    })
}

/// マニフェストから機能情報を抽出
pub fn extract_capabilities(manifest: &ManifestFile) -> PluginCapabilities {
    PluginCapabilities {
        has_settings_ui: manifest.capabilities.has_settings_ui,
        supports_hot_reload: manifest.capabilities.supports_hot_reload,
        supports_async: manifest.capabilities.supports_async,
        supports_gpu: manifest.capabilities.supports_gpu,
        thread_safe: manifest.capabilities.thread_safe,
    }
}

/// マニフェストから依存関係を抽出
pub fn extract_dependencies(manifest: &ManifestFile) -> Vec<PluginDependency> {
    manifest
        .dependencies
        .iter()
        .map(|(id, info)| PluginDependency {
            id: id.clone(),
            min_version: info.min_version.clone(),
            max_version: info.max_version.clone(),
            required: info.required,
        })
        .collect()
}

/// マニフェストからエントリーポイント関数名を取得
pub fn get_entry_point(manifest: &ManifestFile) -> &str {
    &manifest.plugin.entry_point
}

/// バージョン文字列をタプルに解析
fn parse_version(version_str: &str) -> Result<(u32, u32, u32), PluginError> {
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() != 3 {
        return Err(PluginError::InvalidManifest(format!(
            "無効なバージョン形式: {}。'major.minor.patch'形式が必要です",
            version_str
        )));
    }

    let major = parts[0].parse::<u32>().map_err(|_| {
        PluginError::InvalidManifest(format!("無効なメジャーバージョン: {}", parts[0]))
    })?;

    let minor = parts[1].parse::<u32>().map_err(|_| {
        PluginError::InvalidManifest(format!("無効なマイナーバージョン: {}", parts[1]))
    })?;

    let patch = parts[2].parse::<u32>().map_err(|_| {
        PluginError::InvalidManifest(format!("無効なパッチバージョン: {}", parts[2]))
    })?;

    Ok((major, minor, patch))
}

/// APIバージョン文字列をタプルに解析
fn parse_api_version(version_str: &str) -> Result<(u32, u32), PluginError> {
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() != 2 {
        return Err(PluginError::InvalidManifest(format!(
            "無効なAPIバージョン形式: {}。'major.minor'形式が必要です",
            version_str
        )));
    }

    let major = parts[0].parse::<u32>().map_err(|_| {
        PluginError::InvalidManifest(format!("無効なAPIメジャーバージョン: {}", parts[0]))
    })?;

    let minor = parts[1].parse::<u32>().map_err(|_| {
        PluginError::InvalidManifest(format!("無効なAPIマイナーバージョン: {}", parts[1]))
    })?;

    Ok((major, minor))
}

/// マニフェストを検証
pub fn validate_manifest(manifest_path: &Path) -> ManifestValidationResult {
    let mut result = ManifestValidationResult::new();

    // マニフェストの存在チェック
    if !manifest_path.exists() {
        result.add_error(format!(
            "マニフェストファイルが存在しません: {}",
            manifest_path.display()
        ));
        return result;
    }

    // マニフェストの読み込みと解析
    let content = match fs::read_to_string(manifest_path) {
        Ok(content) => content,
        Err(e) => {
            result.add_error(format!("マニフェストファイルの読み込みエラー: {}", e));
            return result;
        }
    };

    let manifest: Result<ManifestFile, toml::de::Error> = toml::from_str(&content);
    let manifest = match manifest {
        Ok(manifest) => manifest,
        Err(e) => {
            result.add_error(format!("マニフェストファイルの解析エラー: {}", e));
            return result;
        }
    };

    // 必須フィールドのチェック
    if manifest.plugin.id.is_empty() {
        result.add_error("プラグインIDが空です".to_string());
    } else if !is_valid_plugin_id(&manifest.plugin.id) {
        result.add_error(format!(
            "無効なプラグインID: {}。英数字、ドット、ハイフン、アンダースコアのみ使用できます",
            manifest.plugin.id
        ));
    }

    if manifest.plugin.name.is_empty() {
        result.add_error("プラグイン名が空です".to_string());
    }

    if manifest.plugin.version.is_empty() {
        result.add_error("プラグインバージョンが空です".to_string());
    } else if parse_version(&manifest.plugin.version).is_err() {
        result.add_error(format!(
            "無効なバージョン形式: {}。'major.minor.patch'形式が必要です",
            manifest.plugin.version
        ));
    }

    if manifest.plugin.author.is_empty() {
        result.add_warning("プラグイン作者が指定されていません".to_string());
    }

    if manifest.plugin.description.is_empty() {
        result.add_warning("プラグイン説明が指定されていません".to_string());
    }

    // プラグイン種類のチェック
    if PluginType::from_str(&manifest.plugin.plugin_type).is_none() {
        result.add_error(format!(
            "無効なプラグイン種類: {}",
            manifest.plugin.plugin_type
        ));
    }

    // APIバージョンチェック
    if manifest.plugin.api_version.is_empty() {
        result.add_error("APIバージョンが空です".to_string());
    } else if parse_api_version(&manifest.plugin.api_version).is_err() {
        result.add_error(format!(
            "無効なAPIバージョン形式: {}。'major.minor'形式が必要です",
            manifest.plugin.api_version
        ));
    }

    // エントリーポイントチェック
    if manifest.plugin.entry_point.is_empty() {
        result.add_error("エントリーポイントが指定されていません".to_string());
    }

    // 依存関係のチェック
    for (dep_id, dep_info) in &manifest.dependencies {
        if dep_id.is_empty() {
            result.add_error("依存プラグインIDが空です".to_string());
            continue;
        }

        if let Some(min_ver) = &dep_info.min_version {
            if parse_version(min_ver).is_err() {
                result.add_error(format!(
                    "依存関係 {} の無効な最小バージョン: {}",
                    dep_id, min_ver
                ));
            }
        }

        if let Some(max_ver) = &dep_info.max_version {
            if parse_version(max_ver).is_err() {
                result.add_error(format!(
                    "依存関係 {} の無効な最大バージョン: {}",
                    dep_id, max_ver
                ));
            }
        }
    }

    result
}

/// プラグインIDが有効な形式かチェック
fn is_valid_plugin_id(id: &str) -> bool {
    let valid_chars = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_';

    !id.is_empty() && id.chars().all(valid_chars)
}

/// 依存関係にループがないかチェック
pub fn check_circular_dependencies(
    plugin_id: &str,
    dependencies: &HashMap<String, Vec<String>>,
    visited: &mut Vec<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    if path.contains(&plugin_id.to_string()) {
        path.push(plugin_id.to_string());
        return Some(path.clone());
    }

    if visited.contains(&plugin_id.to_string()) {
        return None;
    }

    visited.push(plugin_id.to_string());
    path.push(plugin_id.to_string());

    if let Some(deps) = dependencies.get(plugin_id) {
        for dep in deps {
            if let Some(cycle) = check_circular_dependencies(dep, dependencies, visited, path) {
                return Some(cycle);
            }
        }
    }

    path.pop();
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_manifest(content: &str) -> PathBuf {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.into_temp_path();
        path.to_path_buf()
    }

    #[test]
    fn test_load_valid_manifest() {
        let content = r#"
            [plugin]
            id = "com.example.test-plugin"
            name = "テストプラグイン"
            version = "1.0.0"
            author = "テスト作者"
            description = "テスト用プラグイン"
            type = "effect"
            api_version = "1.0"
            entry_point = "create_plugin"
            
            [dependencies]
            "com.example.other-plugin" = { min-version = "0.5.0", required = true }
            
            [capabilities]
            has_settings_ui = true
            supports_hot_reload = false
            thread_safe = true
        "#;

        let path = create_test_manifest(content);
        let manifest = load_manifest(&path).unwrap();

        assert_eq!(manifest.plugin.id, "com.example.test-plugin");
        assert_eq!(manifest.plugin.name, "テストプラグイン");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert_eq!(manifest.plugin.author, "テスト作者");
        assert_eq!(manifest.plugin.plugin_type, "effect");
        assert_eq!(manifest.plugin.api_version, "1.0");
        assert_eq!(manifest.plugin.entry_point, "create_plugin");

        assert!(
            manifest
                .dependencies
                .contains_key("com.example.other-plugin")
        );
        assert_eq!(
            manifest.dependencies["com.example.other-plugin"].min_version,
            Some("0.5.0".to_string())
        );
        assert!(manifest.dependencies["com.example.other-plugin"].required);

        assert!(manifest.capabilities.has_settings_ui);
        assert!(!manifest.capabilities.supports_hot_reload);
        assert!(manifest.capabilities.thread_safe);
    }

    #[test]
    fn test_extract_metadata() {
        let content = r#"
            [plugin]
            id = "com.example.test-plugin"
            name = "テストプラグイン"
            version = "1.0.0"
            author = "テスト作者"
            description = "テスト用プラグイン"
            type = "effect"
            api_version = "1.0"
            entry_point = "create_plugin"
        "#;

        let path = create_test_manifest(content);
        let manifest = load_manifest(&path).unwrap();
        let metadata = extract_metadata(&manifest).unwrap();

        assert_eq!(metadata.id, "com.example.test-plugin");
        assert_eq!(metadata.name, "テストプラグイン");
        assert_eq!(metadata.version, (1, 0, 0));
        assert_eq!(metadata.author, "テスト作者");
        assert_eq!(metadata.description, "テスト用プラグイン");
        assert_eq!(metadata.plugin_type, PluginType::Effect);
        assert_eq!(metadata.api_version, (1, 0));
    }

    #[test]
    fn test_validate_manifest() {
        let valid_content = r#"
            [plugin]
            id = "com.example.test-plugin"
            name = "テストプラグイン"
            version = "1.0.0"
            author = "テスト作者"
            description = "テスト用プラグイン"
            type = "effect"
            api_version = "1.0"
            entry_point = "create_plugin"
        "#;

        let path = create_test_manifest(valid_content);
        let result = validate_manifest(&path);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());

        let invalid_content = r#"
            [plugin]
            id = ""
            name = "テストプラグイン"
            version = "invalid"
            author = ""
            description = ""
            type = "unknown"
            api_version = "1"
            entry_point = ""
        "#;

        let path = create_test_manifest(invalid_content);
        let result = validate_manifest(&path);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("プラグインIDが空です"))
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("無効なバージョン形式"))
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("無効なプラグイン種類"))
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("無効なAPIバージョン形式"))
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("エントリーポイントが指定されていません"))
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("プラグイン作者が指定されていません"))
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("プラグイン説明が指定されていません"))
        );
    }

    #[test]
    fn test_circular_dependencies() {
        let mut dependencies = HashMap::new();
        dependencies.insert("plugin-a".to_string(), vec!["plugin-b".to_string()]);
        dependencies.insert("plugin-b".to_string(), vec!["plugin-c".to_string()]);
        dependencies.insert("plugin-c".to_string(), vec!["plugin-a".to_string()]);

        let mut visited = Vec::new();
        let mut path = Vec::new();

        let cycle = check_circular_dependencies("plugin-a", &dependencies, &mut visited, &mut path);
        assert!(cycle.is_some());

        let cycle_path = cycle.unwrap();
        assert_eq!(
            cycle_path,
            vec!["plugin-a", "plugin-b", "plugin-c", "plugin-a"]
        );
    }
}
