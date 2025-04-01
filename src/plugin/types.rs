//! プラグインシステムの型定義
//!
//! このモジュールはプラグインシステムの基本的な型定義を提供します。
//! プラグインのステート、種類、機能などの列挙型や構造体を定義しています。

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// プラグインの状態を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// プラグインが検出されたが、まだ読み込まれていない
    Discovered,
    /// プラグインが読み込まれたが、まだ初期化されていない
    Loaded,
    /// プラグインが完全に初期化され、使用可能な状態
    Active,
    /// プラグインが一時的に停止されている
    Paused,
    /// プラグインがシャットダウンされたが、まだアンロードされていない
    Inactive,
    /// プラグインの読み込み、初期化、またはシャットダウン中にエラーが発生した
    Error,
}

/// プラグインの種類を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// エフェクトプラグイン - ビデオまたはオーディオに効果を適用
    Effect,
    /// エクスポータープラグイン - 新しい出力フォーマットをサポート
    Exporter,
    /// インポータープラグイン - 新しい入力フォーマットをサポート
    Importer,
    /// UIプラグイン - ユーザーインターフェースをカスタマイズ
    UserInterface,
    /// カスタムプラグイン - 上記以外の拡張機能
    Custom,
}

impl PluginType {
    /// 種類の文字列表現を取得
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginType::Effect => "effect",
            PluginType::Exporter => "exporter",
            PluginType::Importer => "importer",
            PluginType::UserInterface => "ui",
            PluginType::Custom => "custom",
        }
    }

    /// 文字列からプラグイン種類を解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "effect" => Some(PluginType::Effect),
            "exporter" => Some(PluginType::Exporter),
            "importer" => Some(PluginType::Importer),
            "ui" | "userinterface" => Some(PluginType::UserInterface),
            "custom" => Some(PluginType::Custom),
            _ => None,
        }
    }
}

/// プラグイン機能フラグ
#[derive(Debug, Clone, Default)]
pub struct PluginCapabilities {
    /// 設定UIをサポートしているか
    pub has_settings_ui: bool,
    /// ホットリロード（実行中の再読み込み）をサポートしているか
    pub supports_hot_reload: bool,
    /// 非同期処理をサポートしているか
    pub supports_async: bool,
    /// GPUアクセラレーションをサポートしているか
    pub supports_gpu: bool,
    /// 複数のスレッドで安全に実行できるか
    pub thread_safe: bool,
}

/// プラグインのメタデータ
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// プラグインの一意のID
    pub id: String,
    /// プラグインの名前
    pub name: String,
    /// プラグインのバージョン (major, minor, patch)
    pub version: (u32, u32, u32),
    /// プラグイン作者情報
    pub author: String,
    /// プラグインの説明
    pub description: String,
    /// プラグインの種類
    pub plugin_type: PluginType,
    /// プラグインのAPIバージョン
    pub api_version: (u32, u32),
}

impl PluginMetadata {
    /// バージョン文字列を取得
    pub fn version_string(&self) -> String {
        let (major, minor, patch) = self.version;
        format!("{}.{}.{}", major, minor, patch)
    }

    /// APIバージョン文字列を取得
    pub fn api_version_string(&self) -> String {
        let (major, minor) = self.api_version;
        format!("{}.{}", major, minor)
    }
}

/// 読み込まれたプラグインの情報
#[derive(Debug)]
pub struct LoadedPlugin {
    /// プラグインのメタデータ
    pub metadata: PluginMetadata,
    /// プラグインの状態
    pub state: PluginState,
    /// プラグインの機能
    pub capabilities: PluginCapabilities,
    /// プラグインのディレクトリパス
    pub directory: PathBuf,
    /// プラグインのライブラリパス
    pub library_path: PathBuf,
    /// プラグインのマニフェストパス
    pub manifest_path: PathBuf,
    /// プラグインインスタンス（アクティブな場合のみ）
    pub instance: Option<Box<dyn Plugin>>,
    /// エラーメッセージ（エラー状態の場合）
    pub error: Option<String>,
    /// 読み込み時刻
    pub load_time: std::time::SystemTime,
}

/// 基本的なプラグインインターフェース
pub trait Plugin: Send + Sync + 'static {
    /// プラグインの一意のID
    fn id(&self) -> &str;

    /// プラグインの名前
    fn name(&self) -> &str;

    /// プラグインのバージョン
    fn version(&self) -> (u32, u32, u32);

    /// プラグイン作者情報
    fn author(&self) -> &str;

    /// プラグインの説明
    fn description(&self) -> &str;

    /// プラグインの初期化
    fn initialize(&mut self, host: Box<dyn PluginHost>) -> Result<(), super::PluginError>;

    /// プラグインのシャットダウン
    fn shutdown(&mut self) -> Result<(), super::PluginError>;

    /// プラグインの種類
    fn plugin_type(&self) -> PluginType;

    /// プラグインのAPIバージョン
    fn api_version(&self) -> (u32, u32);

    /// プラグインの依存関係
    fn dependencies(&self) -> Vec<super::PluginDependency> {
        Vec::new() // デフォルトでは依存関係なし
    }

    /// プラグインの設定
    fn settings(&self) -> Option<PluginSettings> {
        None // デフォルトでは設定なし
    }

    /// プラグインの機能
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    /// プラグインインスタンスを特定の型にダウンキャスト
    fn as_any(&self) -> &dyn Any;

    /// プラグインインスタンスを特定の型にダウンキャスト（可変）
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// ビデオエフェクトプラグインとしてアクセス
    fn as_video_effect(&self) -> Option<&dyn VideoEffectPlugin> {
        None
    }

    /// オーディオエフェクトプラグインとしてアクセス
    fn as_audio_effect(&self) -> Option<&dyn AudioEffectPlugin> {
        None
    }

    /// エクスポータープラグインとしてアクセス
    fn as_exporter(&self) -> Option<&dyn ExporterPlugin> {
        None
    }

    /// インポータープラグインとしてアクセス
    fn as_importer(&self) -> Option<&dyn ImporterPlugin> {
        None
    }

    /// UIプラグインとしてアクセス
    fn as_ui(&self) -> Option<&dyn UserInterfacePlugin> {
        None
    }
}

/// プラグインホストのインターフェース
pub trait PluginHost: Send + Sync + 'static {
    /// ホストアプリケーションのバージョンを取得
    fn host_version(&self) -> (u32, u32, u32);

    /// ホストアプリケーションの名前を取得
    fn host_name(&self) -> &str;

    /// ロガーを取得
    fn logger(&self) -> &dyn PluginLogger;

    /// 現在のプロジェクトを取得
    fn current_project(&self) -> Option<&dyn ProjectAccess>;

    /// プラグインの作業ディレクトリを取得
    fn plugin_data_dir(&self, plugin_id: &str) -> Result<PathBuf, super::PluginError>;

    /// 設定を取得
    fn get_setting(&self, key: &str) -> Result<Option<SettingValue>, super::PluginError>;

    /// 設定を保存
    fn save_setting(&self, key: &str, value: SettingValue) -> Result<(), super::PluginError>;

    /// 他のプラグインにアクセス
    fn get_plugin(&self, plugin_id: &str) -> Option<&dyn Plugin>;

    /// ユーザーに通知
    fn notify_user(
        &self,
        message: &str,
        notification_type: NotificationType,
    ) -> Result<(), super::PluginError>;

    /// アプリケーションイベントの購読
    fn subscribe_to_event(
        &self,
        event_type: ApplicationEventType,
        callback: Box<dyn Fn(&ApplicationEvent) + Send + Sync>,
    ) -> EventSubscriptionId;

    /// イベント購読の解除
    fn unsubscribe_from_event(
        &self,
        subscription_id: EventSubscriptionId,
    ) -> Result<(), super::PluginError>;

    /// ファイルを開く
    fn open_file(&self, path: &Path, mode: FileMode) -> Result<FileHandle, super::PluginError>;

    /// HTTPリクエストを送信
    fn http_request(&self, url: &str, method: &str) -> Result<HttpResponse, super::PluginError>;
}

/// プラグインのログ機能
pub trait PluginLogger: Send + Sync + 'static {
    /// デバッグレベルのログを記録
    fn debug(&self, message: &str);

    /// 情報レベルのログを記録
    fn info(&self, message: &str);

    /// 警告レベルのログを記録
    fn warn(&self, message: &str);

    /// エラーレベルのログを記録
    fn error(&self, message: &str);
}

/// プロジェクトアクセスインターフェース
pub trait ProjectAccess: Send + Sync + 'static {
    /// プロジェクト名を取得
    fn name(&self) -> &str;

    /// タイムラインを取得
    fn timeline(&self) -> &dyn TimelineAccess;

    /// アセットを取得
    fn assets(&self) -> &dyn AssetAccess;

    /// プロジェクト設定を取得
    fn settings(&self) -> &ProjectSettings;

    /// プロジェクトを保存
    fn save(&self) -> Result<(), ProjectError>;

    /// レンダリングを開始
    fn start_render(&self, config: RenderConfig) -> Result<RenderJobHandle, ProjectError>;
}

/// アプリケーションイベントの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplicationEventType {
    /// プロジェクトが開かれた
    ProjectOpened,
    /// プロジェクトが閉じられた
    ProjectClosed,
    /// プロジェクトが保存された
    ProjectSaved,
    /// アプリケーションが終了する
    ApplicationExit,
    /// レンダリングが開始された
    RenderStarted,
    /// レンダリングが完了した
    RenderCompleted,
    /// レンダリングがキャンセルされた
    RenderCancelled,
    /// タイムラインが変更された
    TimelineChanged,
    /// アセットが追加された
    AssetAdded,
    /// アセットが削除された
    AssetRemoved,
    /// アセットが変更された
    AssetModified,
    /// 設定が変更された
    SettingsChanged,
}

/// アプリケーションイベント
#[derive(Debug, Clone)]
pub struct ApplicationEvent {
    /// イベントの種類
    pub event_type: ApplicationEventType,
    /// イベントの詳細データ
    pub data: HashMap<String, String>,
    /// イベントの発生時刻
    pub timestamp: std::time::SystemTime,
}

/// イベント購読ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventSubscriptionId(pub(crate) u64);

/// 通知の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    /// 情報通知
    Info,
    /// 警告通知
    Warning,
    /// エラー通知
    Error,
    /// 成功通知
    Success,
}

/// ファイルモード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// 読み取り専用
    Read,
    /// 書き込み（既存なら上書き）
    Write,
    /// 追記
    Append,
}

/// ファイルハンドル
pub struct FileHandle {
    pub(crate) inner: Box<dyn std::io::Read + std::io::Write + std::io::Seek + Send + Sync>,
}

/// HTTP応答
pub struct HttpResponse {
    /// ステータスコード
    pub status: u16,
    /// レスポンスヘッダー
    pub headers: HashMap<String, String>,
    /// レスポンスボディ
    pub body: Vec<u8>,
}

/// タイムラインアクセスインターフェース
pub trait TimelineAccess: Send + Sync + 'static {
    // タイムライン関連のメソッド
}

/// アセットアクセスインターフェース
pub trait AssetAccess: Send + Sync + 'static {
    // アセット関連のメソッド
}

/// プロジェクト設定
pub struct ProjectSettings {
    // プロジェクト設定フィールド
}

/// プロジェクトエラー
#[derive(Debug)]
pub enum ProjectError {
    /// 一般的なエラー
    General(String),
}

/// レンダリング設定
pub struct RenderConfig {
    // レンダリング設定フィールド
}

/// レンダリングジョブハンドル
pub struct RenderJobHandle {
    // ジョブハンドルフィールド
}

/// 設定項目の種類
#[derive(Debug, Clone)]
pub enum SettingItemType {
    Boolean,
    Integer,
    Float,
    String,
    Enum(Vec<String>),
    Color,
    FilePath,
    DirectoryPath,
}

/// 設定値
#[derive(Debug, Clone)]
pub enum SettingValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Enum(String),
    Color(u8, u8, u8, u8), // RGBA
    FilePath(PathBuf),
    DirectoryPath(PathBuf),
}

/// 設定項目
#[derive(Debug, Clone)]
pub struct SettingItem {
    /// 設定キー
    pub key: String,
    /// 表示名
    pub display_name: String,
    /// 説明
    pub description: String,
    /// 設定タイプ
    pub item_type: SettingItemType,
    /// デフォルト値
    pub default_value: SettingValue,
    /// 設定が必須かどうか
    pub required: bool,
}

/// プラグイン設定
#[derive(Debug, Clone)]
pub struct PluginSettings {
    /// 設定項目のリスト
    pub items: Vec<SettingItem>,
}

/// ビデオエフェクトプラグイン
pub trait VideoEffectPlugin: Plugin {
    /// エフェクトを適用
    fn apply_effect(
        &self,
        frame: &mut VideoFrame,
        parameters: &HashMap<String, EffectParameter>,
        context: &EffectContext,
    ) -> Result<(), super::PluginError>;

    /// サポートするエフェクトパラメータを取得
    fn get_parameters(&self) -> Vec<EffectParameterDefinition>;

    /// エフェクトのプレビューを生成（オプション）
    fn generate_preview(
        &self,
        parameters: &HashMap<String, EffectParameter>,
        size: (u32, u32),
    ) -> Option<Result<VideoFrame, super::PluginError>> {
        None
    }

    /// GPU加速をサポートしているかどうか
    fn supports_gpu_acceleration(&self) -> bool {
        false
    }

    /// GPU加速版のエフェクト適用関数（GPU加速をサポートする場合）
    fn apply_effect_gpu(
        &self,
        frame: &mut GpuVideoFrame,
        parameters: &HashMap<String, EffectParameter>,
        context: &EffectContext,
    ) -> Result<(), super::PluginError> {
        Err(super::PluginError::NotImplemented)
    }
}

/// オーディオエフェクトプラグイン
pub trait AudioEffectPlugin: Plugin {
    /// エフェクトを適用
    fn apply_effect(
        &self,
        buffer: &mut AudioBuffer,
        parameters: &HashMap<String, EffectParameter>,
        context: &EffectContext,
    ) -> Result<(), super::PluginError>;

    /// サポートするエフェクトパラメータを取得
    fn get_parameters(&self) -> Vec<EffectParameterDefinition>;
}

/// エクスポータープラグイン
pub trait ExporterPlugin: Plugin {
    /// サポートするフォーマットを取得
    fn supported_formats(&self) -> Vec<ExportFormat>;

    /// エクスポートオプションを取得
    fn export_options(&self, format: &str) -> Result<Vec<ExportOption>, super::PluginError>;

    /// エクスポートを開始
    fn begin_export(
        &mut self,
        target_path: &Path,
        format: &str,
        options: &HashMap<String, ExportOptionValue>,
        context: &ExportContext,
    ) -> Result<(), super::PluginError>;

    /// フレームをエクスポート
    fn export_frame(
        &mut self,
        frame: &VideoFrame,
        timestamp: f64,
    ) -> Result<(), super::PluginError>;

    /// オーディオバッファをエクスポート
    fn export_audio(
        &mut self,
        buffer: &AudioBuffer,
        timestamp: f64,
    ) -> Result<(), super::PluginError>;

    /// エクスポートを終了
    fn end_export(&mut self) -> Result<(), super::PluginError>;

    /// エクスポートの進捗状況を取得
    fn export_progress(&self) -> f32;

    /// エクスポートをキャンセル
    fn cancel_export(&mut self) -> Result<(), super::PluginError>;
}

/// インポータープラグイン
pub trait ImporterPlugin: Plugin {
    /// サポートするフォーマットを取得
    fn supported_formats(&self) -> Vec<ImportFormat>;

    /// ファイルがサポートされているかを確認
    fn can_import_file(&self, path: &Path) -> bool;

    /// ファイルからメディア情報を取得
    fn get_media_info(&self, path: &Path) -> Result<MediaInfo, super::PluginError>;

    /// ビデオフレームを読み込む
    fn read_video_frame(
        &mut self,
        path: &Path,
        timestamp: f64,
    ) -> Result<Option<VideoFrame>, super::PluginError>;

    /// オーディオバッファを読み込む
    fn read_audio_buffer(
        &mut self,
        path: &Path,
        start_time: f64,
        duration: f64,
    ) -> Result<Option<AudioBuffer>, super::PluginError>;

    /// インポートをキャンセル
    fn cancel_import(&mut self) -> Result<(), super::PluginError>;
}

/// UIプラグイン
pub trait UserInterfacePlugin: Plugin {
    /// UIコンポーネントを登録
    fn register_components(
        &self,
        registry: &mut dyn UiComponentRegistry,
    ) -> Result<(), super::PluginError>;

    /// UIテーマを提供
    fn provide_theme(&self) -> Option<UiTheme> {
        None
    }

    /// UIイベントハンドラを登録
    fn register_event_handlers(
        &self,
        registry: &mut dyn UiEventRegistry,
    ) -> Result<(), super::PluginError>;

    /// カスタムUIパネルを提供
    fn provide_panels(&self) -> Vec<UiPanelDefinition>;

    /// メニュー項目を提供
    fn provide_menu_items(&self) -> Vec<MenuItem>;
}

// メディア関連の型

/// ビデオフレーム
pub struct VideoFrame {
    // ビデオフレームのフィールド
}

/// GPU対応ビデオフレーム
pub struct GpuVideoFrame {
    // GPU対応ビデオフレームのフィールド
}

/// オーディオバッファ
pub struct AudioBuffer {
    // オーディオバッファのフィールド
}

/// エフェクトコンテキスト
pub struct EffectContext {
    // エフェクトコンテキストのフィールド
}

/// ピクセル
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// エフェクトパラメータの種類
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectParameterType {
    Boolean,
    Integer,
    Float,
    Color,
    Choice,
    Point,
    Range,
}

/// エフェクトパラメータの値
#[derive(Debug, Clone)]
pub enum EffectParameter {
    Boolean(bool),
    Integer(i32),
    Float(f32),
    Color(u8, u8, u8, u8),
    Choice(String),
    Point(f32, f32),
    Range(f32, f32),
}

/// エフェクトパラメータの定義
#[derive(Debug, Clone)]
pub struct EffectParameterDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub parameter_type: EffectParameterType,
    pub default_value: EffectParameter,
    pub min_value: Option<EffectParameter>,
    pub max_value: Option<EffectParameter>,
    pub step: Option<EffectParameter>,
}

/// エクスポートフォーマット
#[derive(Debug, Clone)]
pub struct ExportFormat {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file_extensions: Vec<String>,
}

/// エクスポートオプション
#[derive(Debug, Clone)]
pub struct ExportOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub option_type: SettingItemType,
    pub default_value: ExportOptionValue,
}

/// エクスポートオプション値
#[derive(Debug, Clone)]
pub enum ExportOptionValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Enum(String),
}

/// エクスポートコンテキスト
pub struct ExportContext {
    // エクスポートコンテキストのフィールド
}

/// インポートフォーマット
#[derive(Debug, Clone)]
pub struct ImportFormat {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file_extensions: Vec<String>,
}

/// メディア情報
#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<f64>,
    pub frame_rate: Option<f64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub audio_channels: Option<u32>,
    pub audio_sample_rate: Option<u32>,
    pub codec: Option<String>,
}

/// UIコンポーネントレジストリ
pub trait UiComponentRegistry: Send + Sync {
    fn register_component(
        &mut self,
        id: &str,
        component: Box<dyn Any + Send + Sync>,
    ) -> Result<(), super::PluginError>;
}

/// UIイベントレジストリ
pub trait UiEventRegistry: Send + Sync {
    fn register_handler(
        &mut self,
        event_id: &str,
        handler: Box<dyn Fn(&UiEvent) + Send + Sync>,
    ) -> Result<(), super::PluginError>;
}

/// UIイベント
pub struct UiEvent {
    // UIイベントのフィールド
}

/// UIテーマ
pub struct UiTheme {
    // UIテーマのフィールド
}

/// UIパネル定義
#[derive(Debug, Clone)]
pub struct UiPanelDefinition {
    pub id: String,
    pub title: String,
    pub component_id: String,
    pub default_position: PanelPosition,
    pub default_size: (u32, u32),
    pub min_size: Option<(u32, u32)>,
    pub icon: Option<String>,
}

/// パネル位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPosition {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Float,
}

/// メニュー項目
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub shortcut: Option<String>,
    pub icon: Option<String>,
    pub action: MenuAction,
}

/// メニューアクション
#[derive(Debug, Clone)]
pub enum MenuAction {
    Callback(String),
    TogglePanel(String),
    OpenUrl(String),
    Command(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type() {
        assert_eq!(PluginType::from_str("effect"), Some(PluginType::Effect));
        assert_eq!(PluginType::from_str("ui"), Some(PluginType::UserInterface));
        assert_eq!(PluginType::from_str("unknown"), None);

        assert_eq!(PluginType::Effect.as_str(), "effect");
        assert_eq!(PluginType::UserInterface.as_str(), "ui");
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata {
            id: "test.plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: (1, 2, 3),
            author: "Test Author".to_string(),
            description: "Test Description".to_string(),
            plugin_type: PluginType::Effect,
            api_version: (1, 0),
        };

        assert_eq!(metadata.version_string(), "1.2.3");
        assert_eq!(metadata.api_version_string(), "1.0");
    }
}
