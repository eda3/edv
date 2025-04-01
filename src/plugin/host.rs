//! プラグインホストモジュール
//!
//! プラグインホストは、プラグインがアプリケーションの機能にアクセスするための
//! インターフェースを提供します。ホストAPIを通じて、プラグインはログ出力、
//! プロジェクトデータへのアクセス、設定の管理などを行うことができます。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

use log::{debug, error, info, warn};
use uuid::Uuid;

use super::PluginError;
use super::security::PermissionChecker;
use super::types::{
    ApplicationEvent, ApplicationEventType, EventSubscriptionId, FileHandle, FileMode,
    HttpResponse, NotificationType, Plugin, PluginHost, PluginLogger, ProjectAccess, ProjectError,
    ProjectSettings, SettingValue,
};

/// プラグインイベントハンドラ
type EventHandler = Box<dyn Fn(&ApplicationEvent) + Send + Sync>;

/// イベント購読情報
struct EventSubscription {
    /// 購読ID
    id: EventSubscriptionId,
    /// プラグインID
    plugin_id: String,
    /// イベントタイプ
    event_type: ApplicationEventType,
    /// イベントハンドラ
    handler: EventHandler,
}

/// プラグインホスト実装
pub struct HostImpl {
    /// ホストアプリケーション名
    app_name: String,
    /// ホストアプリケーションバージョン
    app_version: (u32, u32, u32),
    /// プラグインデータディレクトリのベースパス
    plugin_data_base_dir: PathBuf,
    /// 現在のプロジェクト（オプション）
    project: RwLock<Option<Arc<dyn ProjectAccess + Send + Sync>>>,
    /// プラグインごとの設定
    plugin_settings: RwLock<HashMap<String, HashMap<String, SettingValue>>>,
    /// イベント購読
    event_subscriptions: RwLock<Vec<EventSubscription>>,
    /// プラグインレジストリへの参照（弱参照）
    plugin_registry: RwLock<Option<std::sync::Weak<dyn PluginRegistryInterface + Send + Sync>>>,
    /// セキュリティチェッカー
    permission_checker: Arc<PermissionChecker>,
    /// 通知コールバック
    notification_callback: Arc<dyn Fn(&str, NotificationType) + Send + Sync>,
    /// HTTPリクエストハンドラ
    http_handler: Arc<dyn Fn(&str, &str) -> Result<HttpResponse, String> + Send + Sync>,
}

/// プラグインレジストリインターフェース
pub trait PluginRegistryInterface {
    /// プラグインを取得
    fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin>;
}

impl HostImpl {
    /// 新しいプラグインホスト実装を作成
    pub fn new(
        app_name: String,
        app_version: (u32, u32, u32),
        plugin_data_base_dir: PathBuf,
        permission_checker: Arc<PermissionChecker>,
        notification_callback: impl Fn(&str, NotificationType) + Send + Sync + 'static,
        http_handler: impl Fn(&str, &str) -> Result<HttpResponse, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            app_name,
            app_version,
            plugin_data_base_dir,
            project: RwLock::new(None),
            plugin_settings: RwLock::new(HashMap::new()),
            event_subscriptions: RwLock::new(Vec::new()),
            plugin_registry: RwLock::new(None),
            permission_checker,
            notification_callback: Arc::new(notification_callback),
            http_handler: Arc::new(http_handler),
        }
    }

    /// プラグインレジストリを設定
    pub fn set_plugin_registry(&self, registry: Arc<dyn PluginRegistryInterface + Send + Sync>) {
        let mut plugin_registry = self.plugin_registry.write().unwrap();
        *plugin_registry = Some(Arc::downgrade(&registry));
    }

    /// 現在のプロジェクトを設定
    pub fn set_project(&self, project: Option<Arc<dyn ProjectAccess + Send + Sync>>) {
        let mut current_project = self.project.write().unwrap();
        *current_project = project;

        // プロジェクト変更イベントを発火
        let event_type = if current_project.is_some() {
            ApplicationEventType::ProjectOpened
        } else {
            ApplicationEventType::ProjectClosed
        };

        self.fire_event(event_type, HashMap::new());
    }

    /// イベントを発火
    pub fn fire_event(&self, event_type: ApplicationEventType, data: HashMap<String, String>) {
        let event = ApplicationEvent {
            event_type,
            data,
            timestamp: SystemTime::now(),
        };

        let subscriptions = self.event_subscriptions.read().unwrap();
        for sub in subscriptions.iter().filter(|s| s.event_type == event_type) {
            (sub.handler)(&event);
        }
    }

    /// プラグイン設定を保存
    pub fn save_plugin_settings(&self) -> Result<(), PluginError> {
        // 実際のアプリケーションでは、設定を永続化するコードをここに実装
        // 例: ファイルやデータベースに保存

        Ok(())
    }

    /// プラグイン設定をロード
    pub fn load_plugin_settings(&self) -> Result<(), PluginError> {
        // 実際のアプリケーションでは、設定を読み込むコードをここに実装
        // 例: ファイルやデータベースから読み込み

        Ok(())
    }
}

/// プラグインホストの単一インスタンスを作成するためのファクトリ
pub struct HostFactory;

impl HostFactory {
    /// デフォルトのホストを作成
    pub fn create_default_host() -> Arc<HostImpl> {
        let app_data_dir = Self::get_app_data_dir();
        let plugins_data_dir = app_data_dir.join("plugins");

        // ディレクトリが存在しない場合は作成
        if !plugins_data_dir.exists() {
            std::fs::create_dir_all(&plugins_data_dir)
                .expect("プラグインデータディレクトリを作成できません");
        }

        let permission_checker = Arc::new(PermissionChecker::new());

        Arc::new(HostImpl::new(
            "EDV".to_string(),
            (1, 0, 0), // アプリケーションバージョン
            plugins_data_dir,
            permission_checker,
            |message, notification_type| {
                // デフォルトの通知ハンドラ
                match notification_type {
                    NotificationType::Info => info!("プラグイン通知: {}", message),
                    NotificationType::Warning => warn!("プラグイン警告: {}", message),
                    NotificationType::Error => error!("プラグインエラー: {}", message),
                    NotificationType::Success => info!("プラグイン成功: {}", message),
                }
            },
            |url, method| {
                // デフォルトのHTTPハンドラ
                Err("HTTPリクエストはデフォルトでは無効です".to_string())
            },
        ))
    }

    /// アプリケーションデータディレクトリを取得
    fn get_app_data_dir() -> PathBuf {
        let home = dirs::home_dir().expect("ホームディレクトリが見つかりません");

        if cfg!(target_os = "windows") {
            home.join("AppData").join("Local").join("EDV")
        } else if cfg!(target_os = "macos") {
            home.join("Library").join("Application Support").join("EDV")
        } else {
            home.join(".local").join("share").join("edv")
        }
    }
}

/// プラグインのロガー実装
#[derive(Clone)]
pub struct PluginLoggerImpl {
    /// プラグインID
    plugin_id: String,
}

impl PluginLoggerImpl {
    /// 新しいロガーを作成
    pub fn new(plugin_id: String) -> Self {
        Self { plugin_id }
    }
}

impl PluginLogger for PluginLoggerImpl {
    fn debug(&self, message: &str) {
        debug!("[Plugin: {}] {}", self.plugin_id, message);
    }

    fn info(&self, message: &str) {
        info!("[Plugin: {}] {}", self.plugin_id, message);
    }

    fn warn(&self, message: &str) {
        warn!("[Plugin: {}] {}", self.plugin_id, message);
    }

    fn error(&self, message: &str) {
        error!("[Plugin: {}] {}", self.plugin_id, message);
    }
}

/// プラグインごとのホストインスタンス
pub struct PluginHostInstance {
    /// プラグインID
    plugin_id: String,
    /// ロガー
    logger: Arc<PluginLoggerImpl>,
    /// ホスト実装への参照
    host_impl: Arc<HostImpl>,
}

impl PluginHostInstance {
    /// 新しいプラグインホストインスタンスを作成
    pub fn new(plugin_id: String, host_impl: Arc<HostImpl>) -> Self {
        let logger = Arc::new(PluginLoggerImpl::new(plugin_id.clone()));

        Self {
            plugin_id,
            logger,
            host_impl,
        }
    }
}

impl PluginHost for PluginHostInstance {
    fn host_version(&self) -> (u32, u32, u32) {
        self.host_impl.app_version
    }

    fn host_name(&self) -> &str {
        &self.host_impl.app_name
    }

    fn logger(&self) -> &dyn PluginLogger {
        self.logger.as_ref()
    }

    fn current_project(&self) -> Option<&dyn ProjectAccess> {
        let project = self.host_impl.project.read().unwrap();
        project.as_ref().map(|p| p.as_ref() as &dyn ProjectAccess)
    }

    fn plugin_data_dir(&self, plugin_id: &str) -> Result<PathBuf, PluginError> {
        // プラグインは自分自身のデータディレクトリにのみアクセス可能
        if plugin_id != self.plugin_id {
            return Err(PluginError::PermissionDenied(format!(
                "プラグイン {} は {} のデータディレクトリにアクセスできません",
                self.plugin_id, plugin_id
            )));
        }

        let plugin_dir = self.host_impl.plugin_data_base_dir.join(plugin_id);

        // ディレクトリが存在しない場合は作成
        if !plugin_dir.exists() {
            std::fs::create_dir_all(&plugin_dir).map_err(|e| {
                PluginError::IO(format!("プラグインデータディレクトリの作成に失敗: {}", e))
            })?;
        }

        Ok(plugin_dir)
    }

    fn get_setting(&self, key: &str) -> Result<Option<SettingValue>, PluginError> {
        let settings = self.host_impl.plugin_settings.read().unwrap();

        if let Some(plugin_settings) = settings.get(&self.plugin_id) {
            Ok(plugin_settings.get(key).cloned())
        } else {
            Ok(None)
        }
    }

    fn save_setting(&self, key: &str, value: SettingValue) -> Result<(), PluginError> {
        let mut settings = self.host_impl.plugin_settings.write().unwrap();

        let plugin_settings = settings
            .entry(self.plugin_id.clone())
            .or_insert_with(HashMap::new);
        plugin_settings.insert(key.to_string(), value);

        // 設定変更を永続化
        drop(settings); // ロックを解放
        self.host_impl.save_plugin_settings()
    }

    fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        // プラグインのアクセス権をチェック
        if !self
            .host_impl
            .permission_checker
            .can_access_plugin(&self.plugin_id, plugin_id)
        {
            self.logger.warn(&format!(
                "プラグイン {} へのアクセスが拒否されました",
                plugin_id
            ));
            return None;
        }

        let registry = self.host_impl.plugin_registry.read().unwrap();
        if let Some(registry_weak) = &*registry {
            if let Some(registry) = registry_weak.upgrade() {
                return registry.get_plugin(plugin_id);
            }
        }

        None
    }

    fn notify_user(
        &self,
        message: &str,
        notification_type: NotificationType,
    ) -> Result<(), PluginError> {
        // 通知権限をチェック
        if !self
            .host_impl
            .permission_checker
            .can_send_notifications(&self.plugin_id)
        {
            return Err(PluginError::PermissionDenied(format!(
                "プラグイン {} は通知を送信する権限がありません",
                self.plugin_id
            )));
        }

        (self.host_impl.notification_callback)(message, notification_type);
        Ok(())
    }

    fn subscribe_to_event(
        &self,
        event_type: ApplicationEventType,
        callback: Box<dyn Fn(&ApplicationEvent) + Send + Sync>,
    ) -> EventSubscriptionId {
        let id = EventSubscriptionId(Uuid::new_v4().as_u128() as u64);

        let subscription = EventSubscription {
            id,
            plugin_id: self.plugin_id.clone(),
            event_type,
            handler: callback,
        };

        let mut subscriptions = self.host_impl.event_subscriptions.write().unwrap();
        subscriptions.push(subscription);

        id
    }

    fn unsubscribe_from_event(
        &self,
        subscription_id: EventSubscriptionId,
    ) -> Result<(), PluginError> {
        let mut subscriptions = self.host_impl.event_subscriptions.write().unwrap();

        let index = subscriptions
            .iter()
            .position(|s| s.id == subscription_id && s.plugin_id == self.plugin_id);

        if let Some(index) = index {
            subscriptions.remove(index);
            Ok(())
        } else {
            Err(PluginError::InvalidOperation(format!(
                "購読 {} が見つかりません",
                subscription_id.0
            )))
        }
    }

    fn open_file(&self, path: &Path, mode: FileMode) -> Result<FileHandle, PluginError> {
        // ファイルアクセス権限をチェック
        if !self
            .host_impl
            .permission_checker
            .can_access_file(&self.plugin_id, path, &mode)
        {
            return Err(PluginError::PermissionDenied(format!(
                "プラグイン {} はファイル {} へのアクセスが拒否されました",
                self.plugin_id,
                path.display()
            )));
        }

        use std::fs::OpenOptions;
        use std::io::{Read, Seek, Write};

        let file = match mode {
            FileMode::Read => OpenOptions::new().read(true).open(path),
            FileMode::Write => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path),
            FileMode::Append => OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(path),
        }
        .map_err(|e| {
            PluginError::IO(format!("ファイルオープンエラー: {}: {}", path.display(), e))
        })?;

        struct FileWrapper {
            file: std::fs::File,
        }

        impl Read for FileWrapper {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.file.read(buf)
            }
        }

        impl Write for FileWrapper {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.file.write(buf)
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.file.flush()
            }
        }

        impl Seek for FileWrapper {
            fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
                self.file.seek(pos)
            }
        }

        Ok(FileHandle {
            inner: Box::new(FileWrapper { file }),
        })
    }

    fn http_request(&self, url: &str, method: &str) -> Result<HttpResponse, PluginError> {
        // ネットワークアクセス権限をチェック
        if !self
            .host_impl
            .permission_checker
            .can_access_network(&self.plugin_id, url)
        {
            return Err(PluginError::PermissionDenied(format!(
                "プラグイン {} は URL {} へのアクセスが拒否されました",
                self.plugin_id, url
            )));
        }

        (self.host_impl.http_handler)(url, method).map_err(|e| PluginError::NetworkError(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_plugin_logger() {
        let logger = PluginLoggerImpl::new("test.plugin".to_string());

        // ロギング操作はエラーを起こさないことを確認
        logger.debug("デバッグメッセージ");
        logger.info("情報メッセージ");
        logger.warn("警告メッセージ");
        logger.error("エラーメッセージ");
    }

    #[test]
    fn test_event_subscription() {
        let host_impl = HostFactory::create_default_host();
        let plugin_host = PluginHostInstance::new("test.plugin".to_string(), host_impl.clone());

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        // イベント購読を設定
        let subscription_id = plugin_host.subscribe_to_event(
            ApplicationEventType::ProjectOpened,
            Box::new(move |_| {
                called_clone.store(true, Ordering::SeqCst);
            }),
        );

        // イベントを発火
        host_impl.fire_event(ApplicationEventType::ProjectOpened, HashMap::new());

        // イベントハンドラが呼び出されたことを確認
        assert!(called.load(Ordering::SeqCst));

        // 購読解除
        let result = plugin_host.unsubscribe_from_event(subscription_id);
        assert!(result.is_ok());

        // 購読解除後はイベントが呼び出されないことを確認
        called.store(false, Ordering::SeqCst);
        host_impl.fire_event(ApplicationEventType::ProjectOpened, HashMap::new());
        assert!(!called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_plugin_settings() {
        let host_impl = HostFactory::create_default_host();
        let plugin_host = PluginHostInstance::new("test.plugin".to_string(), host_impl);

        // 初期状態では設定が存在しないことを確認
        let result = plugin_host.get_setting("test_key");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // 設定を保存
        let value = SettingValue::String("test_value".to_string());
        let result = plugin_host.save_setting("test_key", value.clone());
        assert!(result.is_ok());

        // 保存した設定を取得して確認
        let result = plugin_host.get_setting("test_key");
        assert!(result.is_ok());

        match result.unwrap() {
            Some(SettingValue::String(s)) => assert_eq!(s, "test_value"),
            _ => panic!("期待する設定値ではありません"),
        }
    }
}
