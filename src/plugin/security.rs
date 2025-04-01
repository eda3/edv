//! プラグインセキュリティモジュール
//!
//! プラグインのセキュリティに関する機能を提供します。権限管理、サンドボックス化、
//! リソース制限などの機能を通じて、プラグインが安全に実行されるようにします。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use url::Url;

use log::{debug, info, warn};

use super::types::FileMode;

/// 権限の種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionType {
    /// ファイルシステムへのアクセス
    FileSystem {
        /// アクセス可能なパスのリスト
        paths: Vec<PathBuf>,
        /// 読み取り可能か
        read: bool,
        /// 書き込み可能か
        write: bool,
    },
    /// ネットワークアクセス
    Network {
        /// アクセス可能なドメインのリスト
        domains: Vec<String>,
        /// アクセス可能なURLのリスト
        urls: Vec<String>,
    },
    /// 通知の送信
    Notification,
    /// プラグイン間の相互作用
    PluginInteraction {
        /// アクセス可能なプラグインIDのリスト
        plugin_ids: Vec<String>,
    },
    /// リソース使用量の制限
    ResourceLimit {
        /// メモリ使用量の制限 (バイト)
        memory_limit: Option<usize>,
        /// CPUタイム使用量の制限 (ミリ秒)
        cpu_time_limit: Option<u64>,
    },
}

/// プラグインの権限マニフェスト
#[derive(Debug, Default)]
pub struct PluginPermissions {
    /// 許可された権限のリスト
    permissions: HashSet<PermissionType>,
    /// ユーザーが明示的に許可した追加権限
    user_granted: HashSet<PermissionType>,
}

impl PluginPermissions {
    /// 新しい権限マニフェストを作成
    pub fn new() -> Self {
        Self {
            permissions: HashSet::new(),
            user_granted: HashSet::new(),
        }
    }

    /// デフォルトの権限セットを取得
    pub fn default_permissions() -> HashSet<PermissionType> {
        let mut permissions = HashSet::new();

        // デフォルトでは、プラグイン自身のデータディレクトリにのみアクセス可能
        // 実際の実装では、ここでプラグインのデータディレクトリを設定する

        // 通知の送信は許可
        permissions.insert(PermissionType::Notification);

        permissions
    }

    /// 権限を追加
    pub fn add_permission(&mut self, permission: PermissionType) {
        self.permissions.insert(permission);
    }

    /// ユーザーが許可した権限を追加
    pub fn add_user_granted_permission(&mut self, permission: PermissionType) {
        self.user_granted.insert(permission);
    }

    /// 権限をチェック
    pub fn has_permission(&self, permission: &PermissionType) -> bool {
        self.permissions.contains(permission) || self.user_granted.contains(permission)
    }

    /// ファイルアクセス権限をチェック
    pub fn can_access_file(&self, path: &Path, mode: &FileMode) -> bool {
        let is_write = match mode {
            FileMode::Read => false,
            FileMode::Write | FileMode::Append => true,
        };

        for perm in self.permissions.iter().chain(self.user_granted.iter()) {
            if let PermissionType::FileSystem { paths, read, write } = perm {
                if (is_write && !write) || (!is_write && !read) {
                    continue;
                }

                for allowed_path in paths {
                    if path.starts_with(allowed_path) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// ネットワークアクセス権限をチェック
    pub fn can_access_network(&self, url_str: &str) -> bool {
        // URLを解析
        let url = match Url::parse(url_str) {
            Ok(url) => url,
            Err(_) => return false,
        };

        let host = match url.host_str() {
            Some(host) => host.to_string(),
            None => return false,
        };

        for perm in self.permissions.iter().chain(self.user_granted.iter()) {
            if let PermissionType::Network { domains, urls } = perm {
                // 完全なURLが許可リストにあるかチェック
                if urls.iter().any(|u| u == url_str) {
                    return true;
                }

                // ドメインが許可リストにあるかチェック
                if domains
                    .iter()
                    .any(|d| host == *d || host.ends_with(&format!(".{}", d)))
                {
                    return true;
                }
            }
        }

        false
    }

    /// プラグイン相互作用の権限をチェック
    pub fn can_access_plugin(&self, target_plugin_id: &str) -> bool {
        for perm in self.permissions.iter().chain(self.user_granted.iter()) {
            if let PermissionType::PluginInteraction { plugin_ids } = perm {
                if plugin_ids.iter().any(|id| id == target_plugin_id) {
                    return true;
                }
            }
        }

        false
    }

    /// 通知送信権限をチェック
    pub fn can_send_notifications(&self) -> bool {
        self.permissions.contains(&PermissionType::Notification)
            || self.user_granted.contains(&PermissionType::Notification)
    }
}

/// 権限チェッカー
#[derive(Debug, Default)]
pub struct PermissionChecker {
    /// プラグインごとの権限
    plugin_permissions: RwLock<HashMap<String, PluginPermissions>>,
}

impl PermissionChecker {
    /// 新しい権限チェッカーを作成
    pub fn new() -> Self {
        Self {
            plugin_permissions: RwLock::new(HashMap::new()),
        }
    }

    /// プラグインの権限を設定
    pub fn set_plugin_permissions(&self, plugin_id: &str, permissions: PluginPermissions) {
        let mut permissions_map = self.plugin_permissions.write().unwrap();
        permissions_map.insert(plugin_id.to_string(), permissions);
    }

    /// プラグインの権限を取得
    pub fn get_plugin_permissions(&self, plugin_id: &str) -> Option<PluginPermissions> {
        let permissions_map = self.plugin_permissions.read().unwrap();
        permissions_map.get(plugin_id).cloned()
    }

    /// プラグインの登録
    pub fn register_plugin(&self, plugin_id: &str) {
        let mut permissions_map = self.plugin_permissions.write().unwrap();
        if !permissions_map.contains_key(plugin_id) {
            let mut permissions = PluginPermissions::new();

            // デフォルト権限を設定
            for perm in PluginPermissions::default_permissions() {
                permissions.add_permission(perm);
            }

            permissions_map.insert(plugin_id.to_string(), permissions);
            debug!("プラグイン {} を権限システムに登録しました", plugin_id);
        }
    }

    /// プラグインの削除
    pub fn unregister_plugin(&self, plugin_id: &str) {
        let mut permissions_map = self.plugin_permissions.write().unwrap();
        permissions_map.remove(plugin_id);
        debug!("プラグイン {} を権限システムから削除しました", plugin_id);
    }

    /// ファイルアクセス権限をチェック
    pub fn can_access_file(&self, plugin_id: &str, path: &Path, mode: &FileMode) -> bool {
        let permissions_map = self.plugin_permissions.read().unwrap();

        if let Some(permissions) = permissions_map.get(plugin_id) {
            let result = permissions.can_access_file(path, mode);
            if !result {
                warn!(
                    "プラグイン {} はファイル {} へのアクセスが拒否されました",
                    plugin_id,
                    path.display()
                );
            }
            result
        } else {
            warn!(
                "プラグイン {} は権限システムに登録されていません",
                plugin_id
            );
            false
        }
    }

    /// ネットワークアクセス権限をチェック
    pub fn can_access_network(&self, plugin_id: &str, url: &str) -> bool {
        let permissions_map = self.plugin_permissions.read().unwrap();

        if let Some(permissions) = permissions_map.get(plugin_id) {
            let result = permissions.can_access_network(url);
            if !result {
                warn!(
                    "プラグイン {} は URL {} へのアクセスが拒否されました",
                    plugin_id, url
                );
            }
            result
        } else {
            warn!(
                "プラグイン {} は権限システムに登録されていません",
                plugin_id
            );
            false
        }
    }

    /// プラグイン相互作用の権限をチェック
    pub fn can_access_plugin(&self, plugin_id: &str, target_plugin_id: &str) -> bool {
        let permissions_map = self.plugin_permissions.read().unwrap();

        if let Some(permissions) = permissions_map.get(plugin_id) {
            let result = permissions.can_access_plugin(target_plugin_id);
            if !result {
                warn!(
                    "プラグイン {} はプラグイン {} へのアクセスが拒否されました",
                    plugin_id, target_plugin_id
                );
            }
            result
        } else {
            warn!(
                "プラグイン {} は権限システムに登録されていません",
                plugin_id
            );
            false
        }
    }

    /// 通知送信権限をチェック
    pub fn can_send_notifications(&self, plugin_id: &str) -> bool {
        let permissions_map = self.plugin_permissions.read().unwrap();

        if let Some(permissions) = permissions_map.get(plugin_id) {
            let result = permissions.can_send_notifications();
            if !result {
                warn!("プラグイン {} は通知の送信が拒否されました", plugin_id);
            }
            result
        } else {
            warn!(
                "プラグイン {} は権限システムに登録されていません",
                plugin_id
            );
            false
        }
    }

    /// 権限を追加（ユーザーの明示的な許可が必要な権限）
    pub fn request_permission(
        &self,
        plugin_id: &str,
        permission: PermissionType,
        user_consent_callback: impl Fn(&str, &PermissionType) -> bool,
    ) -> bool {
        let mut permissions_map = self.plugin_permissions.write().unwrap();

        if let Some(permissions) = permissions_map.get_mut(plugin_id) {
            // すでに許可済みかチェック
            if permissions.has_permission(&permission) {
                return true;
            }

            // ユーザーに許可を求める
            let granted = user_consent_callback(plugin_id, &permission);

            if granted {
                permissions.add_user_granted_permission(permission);
                info!("プラグイン {} に追加権限が許可されました", plugin_id);
                true
            } else {
                warn!(
                    "プラグイン {} の追加権限リクエストが拒否されました",
                    plugin_id
                );
                false
            }
        } else {
            warn!(
                "プラグイン {} は権限システムに登録されていません",
                plugin_id
            );
            false
        }
    }

    /// プラグインに安全なデータディレクトリへのアクセス権限を追加
    pub fn grant_data_directory_access(&self, plugin_id: &str, data_dir: &Path) {
        let mut permissions_map = self.plugin_permissions.write().unwrap();

        if let Some(permissions) = permissions_map.get_mut(plugin_id) {
            let fs_permission = PermissionType::FileSystem {
                paths: vec![data_dir.to_path_buf()],
                read: true,
                write: true,
            };

            permissions.add_permission(fs_permission);
            debug!(
                "プラグイン {} にデータディレクトリ {} へのアクセス権限を付与しました",
                plugin_id,
                data_dir.display()
            );
        }
    }
}

/// サンドボックス化されたプラグイン実行環境
pub struct PluginSandbox {
    /// プラグインID
    plugin_id: String,
    /// リソース使用量の監視情報
    resource_usage: RwLock<ResourceUsage>,
    /// 権限チェッカーへの参照
    permission_checker: std::sync::Weak<PermissionChecker>,
}

/// リソース使用量情報
#[derive(Debug, Default)]
struct ResourceUsage {
    /// メモリ使用量（バイト）
    memory_usage: usize,
    /// CPU時間（ミリ秒）
    cpu_time: u64,
    /// ディスク読み取り量（バイト）
    disk_read: u64,
    /// ディスク書き込み量（バイト）
    disk_write: u64,
    /// ネットワーク送信量（バイト）
    network_sent: u64,
    /// ネットワーク受信量（バイト）
    network_received: u64,
}

impl PluginSandbox {
    /// 新しいサンドボックスを作成
    pub fn new(plugin_id: String, permission_checker: std::sync::Weak<PermissionChecker>) -> Self {
        Self {
            plugin_id,
            resource_usage: RwLock::new(ResourceUsage::default()),
            permission_checker,
        }
    }

    /// リソース使用量を記録
    pub fn record_resource_usage(
        &self,
        memory_delta: usize,
        cpu_time_delta: u64,
        disk_read_delta: u64,
        disk_write_delta: u64,
        network_sent_delta: u64,
        network_received_delta: u64,
    ) {
        let mut usage = self.resource_usage.write().unwrap();

        usage.memory_usage += memory_delta;
        usage.cpu_time += cpu_time_delta;
        usage.disk_read += disk_read_delta;
        usage.disk_write += disk_write_delta;
        usage.network_sent += network_sent_delta;
        usage.network_received += network_received_delta;
    }

    /// 現在のリソース使用量を取得
    pub fn get_resource_usage(&self) -> ResourceUsage {
        self.resource_usage.read().unwrap().clone()
    }

    /// リソース制限をチェック
    pub fn check_resource_limits(&self) -> bool {
        if let Some(checker) = self.permission_checker.upgrade() {
            if let Some(permissions) = checker.get_plugin_permissions(&self.plugin_id) {
                let usage = self.resource_usage.read().unwrap();

                for perm in permissions
                    .permissions
                    .iter()
                    .chain(permissions.user_granted.iter())
                {
                    if let PermissionType::ResourceLimit {
                        memory_limit,
                        cpu_time_limit,
                    } = perm
                    {
                        // メモリ制限をチェック
                        if let Some(limit) = memory_limit {
                            if usage.memory_usage > *limit {
                                warn!(
                                    "プラグイン {} はメモリ使用量制限を超えました: {} > {}",
                                    self.plugin_id, usage.memory_usage, limit
                                );
                                return false;
                            }
                        }

                        // CPU時間制限をチェック
                        if let Some(limit) = cpu_time_limit {
                            if usage.cpu_time > *limit {
                                warn!(
                                    "プラグイン {} はCPU時間制限を超えました: {} > {}",
                                    self.plugin_id, usage.cpu_time, limit
                                );
                                return false;
                            }
                        }
                    }
                }
            }
        }

        true
    }

    /// サンドボックスをリセット
    pub fn reset(&self) {
        let mut usage = self.resource_usage.write().unwrap();
        *usage = ResourceUsage::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_permissions() {
        let mut permissions = PluginPermissions::new();

        // 通知権限を追加
        permissions.add_permission(PermissionType::Notification);
        assert!(permissions.can_send_notifications());

        // ファイルアクセス権限を追加
        let temp_dir = std::env::temp_dir();
        let fs_perm = PermissionType::FileSystem {
            paths: vec![temp_dir.clone()],
            read: true,
            write: false,
        };

        permissions.add_permission(fs_perm);

        // 読み取りモードでのアクセスをチェック
        assert!(permissions.can_access_file(&temp_dir.join("test.txt"), &FileMode::Read));

        // 書き込みモードでのアクセスをチェック（拒否されるはず）
        assert!(!permissions.can_access_file(&temp_dir.join("test.txt"), &FileMode::Write));

        // ネットワークアクセス権限を追加
        let net_perm = PermissionType::Network {
            domains: vec!["example.com".to_string()],
            urls: vec!["https://allowed-url.com/resource".to_string()],
        };

        permissions.add_permission(net_perm);

        // 許可されているドメインへのアクセスをチェック
        assert!(permissions.can_access_network("https://example.com/path"));
        assert!(permissions.can_access_network("https://sub.example.com/path"));

        // 許可されているURLへのアクセスをチェック
        assert!(permissions.can_access_network("https://allowed-url.com/resource"));

        // 許可されていないドメインへのアクセスをチェック
        assert!(!permissions.can_access_network("https://not-allowed.com/"));
    }

    #[test]
    fn test_permission_checker() {
        let checker = PermissionChecker::new();

        // プラグインを登録
        checker.register_plugin("test.plugin");

        // プラグインに追加権限を付与
        let temp_dir = std::env::temp_dir();
        checker.grant_data_directory_access("test.plugin", &temp_dir);

        // ファイルアクセス権限をチェック
        assert!(checker.can_access_file(
            "test.plugin",
            &temp_dir.join("file.txt"),
            &FileMode::Read
        ));
        assert!(checker.can_access_file(
            "test.plugin",
            &temp_dir.join("file.txt"),
            &FileMode::Write
        ));

        // 通知権限をチェック（デフォルトで許可）
        assert!(checker.can_send_notifications("test.plugin"));

        // プラグイン間アクセス権限をチェック（デフォルトでは拒否）
        assert!(!checker.can_access_plugin("test.plugin", "other.plugin"));

        // ネットワークアクセス権限をチェック（デフォルトでは拒否）
        assert!(!checker.can_access_network("test.plugin", "https://example.com"));

        // ユーザー許可コールバック
        let user_consent = |plugin_id: &str, permission: &PermissionType| -> bool {
            match permission {
                PermissionType::Network { .. } => true,
                _ => false,
            }
        };

        // ネットワーク権限をリクエスト
        let net_perm = PermissionType::Network {
            domains: vec!["example.com".to_string()],
            urls: vec![],
        };

        let granted = checker.request_permission("test.plugin", net_perm, user_consent);
        assert!(granted);

        // ネットワークアクセス権限をチェック（許可された）
        assert!(checker.can_access_network("test.plugin", "https://example.com/path"));

        // プラグインの登録解除
        checker.unregister_plugin("test.plugin");

        // 登録解除後は権限がないことを確認
        assert!(!checker.can_access_file(
            "test.plugin",
            &temp_dir.join("file.txt"),
            &FileMode::Read
        ));
    }

    #[test]
    fn test_sandbox() {
        let checker = Arc::new(PermissionChecker::new());
        let sandbox = PluginSandbox::new("test.plugin".to_string(), Arc::downgrade(&checker));

        // リソース使用量を記録
        sandbox.record_resource_usage(
            1024 * 1024, // 1MB
            100,         // 100ms
            512,         // 512バイト読み取り
            256,         // 256バイト書き込み
            1024,        // 1KBネットワーク送信
            2048,        // 2KBネットワーク受信
        );

        // 使用量を取得して確認
        let usage = sandbox.get_resource_usage();
        assert_eq!(usage.memory_usage, 1024 * 1024);
        assert_eq!(usage.cpu_time, 100);
        assert_eq!(usage.disk_read, 512);
        assert_eq!(usage.disk_write, 256);
        assert_eq!(usage.network_sent, 1024);
        assert_eq!(usage.network_received, 2048);

        // サンドボックスをリセット
        sandbox.reset();

        // リセット後の使用量を確認
        let usage = sandbox.get_resource_usage();
        assert_eq!(usage.memory_usage, 0);
        assert_eq!(usage.cpu_time, 0);
    }
}
