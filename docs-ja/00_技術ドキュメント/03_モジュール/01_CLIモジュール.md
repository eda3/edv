# edv - CLIモジュール実装

このドキュメントでは、edvアプリケーションのコマンドラインインターフェース（CLI）モジュールの詳細な実装ガイドラインを提供します。

## 概要

CLIモジュールは、edvアプリケーションの主要なユーザーインターフェースとして機能し、コマンドの解析、実行、およびユーザーとの対話を担当します。これにより、ビデオ編集操作のための一貫性のある直感的なコマンドライン体験を提供します。

## 構造

```
src/cli/
├── mod.rs        # モジュールのエクスポート、Errorの列挙型、Result型
├── app.rs        # メインアプリケーションのエントリーポイント（App, Cli, Commands）
├── commands.rs   # コマンドレジストリと実装
├── args.rs       # 引数解析ユーティリティ
├── output.rs     # ターミナル出力の整形と進捗報告
└── utils.rs      # CLIユーティリティ（ヘルプテキスト、バリデーション）
```

## 主要コンポーネント

### App（app.rs）

メインアプリケーションのエントリーポイントとコマンドディスパッチャー：

```rust
/// CLI アプリケーション構造体
pub struct App {
    /// 全ての利用可能なコマンドを含むコマンドレジストリ
    command_registry: CommandRegistry,
    /// アプリケーション設定
    config: Config,
    /// アプリケーションメッセージ用のロガー
    logger: Box<dyn Logger>,
}

impl App {
    /// 指定された設定で新しいアプリケーションインスタンスを作成
    pub fn new(config: Config, logger: Box<dyn Logger>) -> Self {
        Self {
            command_registry: CommandRegistry::new(),
            config,
            logger,
        }
    }
    
    /// アプリケーションを初期化し、全ての利用可能なコマンドを登録
    pub fn initialize(&mut self) -> Result<()> {
        // 全てのコマンドを登録
        self.register_commands()?;
        
        self.logger.info("アプリケーションが初期化されました");
        Ok(())
    }
    
    /// 全ての利用可能なコマンドをコマンドレジストリに登録
    fn register_commands(&mut self) -> Result<()> {
        // コマンドを登録
        self.command_registry.register(Box::new(commands::InfoCommand::new()))?;
        self.command_registry.register(Box::new(commands::RenderCommand::new()))?;
        // 追加のコマンドはここに登録される
        
        Ok(())
    }
    
    /// 指定されたコマンドを引数とともに実行
    pub fn execute_command(&self, command: Commands) -> Result<()> {
        // 実行コンテキストを作成
        let context = self.create_execution_context()?;
        
        // コマンドタイプに基づいて適切なハンドラーを実行
        match command {
            Commands::Trim { input, output, start, end, recompress } => {
                // トリム実装
            },
            Commands::Concat { input, output, recompress } => {
                // 連結実装
            },
            Commands::Info { input, detailed } => {
                // レジストリからInfoCommandを取得して実行
                if let Ok(info_cmd) = self.command_registry.get("info") {
                    // 引数を変換
                    let mut args = vec![input];
                    if detailed {
                        args.push("--detailed".to_string());
                    }
                    
                    // 準備された引数でコマンドを実行
                    info_cmd.execute(&context, &args)?;
                } else {
                    // フォールバックのプレースホルダー実装
                    self.logger.info("情報コマンドが正常に実行されました");
                }
            },
            // その他のコマンド...
        }
        
        Ok(())
    }
    
    /// コマンド実行のための実行コンテキストを作成
    fn create_execution_context(&self) -> Result<Context> {
        Ok(Context::new(self.config.clone(), self.logger.clone()))
    }
}

/// アプリケーションのエントリーポイント
pub fn run() -> Result<()> {
    // コマンドライン引数を解析
    let cli = Cli::parse();
    
    // 詳細度に基づいてロガーを設定
    let log_level = if cli.verbose { LogLevel::Debug } else { LogLevel::Info };
    let logger = Box::new(ConsoleLogger::new(log_level));
    
    // 設定ファイルからロードするか、デフォルトを使用
    let config = match cli.config {
        Some(ref path) => Config::load_from_file(path)?,
        None => Config::load_default()?,
    };
    
    // アプリケーションを作成して初期化
    let mut app = App::new(config, logger);
    app.initialize()?;
    
    // コマンドを実行
    app.execute_command(cli.command)
}
```

### メインエントリーポイント（main.rs）

main関数はアプリケーションのエントリーポイントとして機能し、CLIを実行します：

```rust
use edv::cli;

fn main() {
    // CLIアプリケーションを実行
    if let Err(err) = cli::run() {
        eprintln!("エラー: {}", err);
        std::process::exit(1);
    }
}
```

### コマンドライン解析（app.rs）

CLIモジュールはコマンドライン解析にclapを使用します：

```rust
/// コマンドライン引数パーサー
#[derive(Parser)]
#[clap(
    name = "edv",
    about = "FFmpegベースのCLIビデオ編集ツール",
    version,
    author
)]
pub struct Cli {
    /// 実行するサブコマンド
    #[clap(subcommand)]
    pub command: Commands,

    /// 詳細な出力を有効にする
    #[clap(short, long, global = true)]
    pub verbose: bool,

    /// 設定ファイルへのパス
    #[clap(short, long, global = true)]
    pub config: Option<PathBuf>,
}

/// アプリケーションでサポートされるサブコマンド
#[derive(Subcommand)]
pub enum Commands {
    /// ビデオを指定された開始時間と終了時間にトリムする
    Trim {
        /// 入力ビデオファイルパス
        #[clap(short, long, value_parser)]
        input: String,

        /// 出力ビデオファイルパス
        #[clap(short, long, value_parser)]
        output: String,

        /// 開始時間（形式：HH:MM:SS.mmm または秒）
        #[clap(short, long)]
        start: Option<String>,

        /// 終了時間（形式：HH:MM:SS.mmm または秒）
        #[clap(short, long)]
        end: Option<String>,

        /// ストリームコピーの代わりにビデオを再エンコードする
        #[clap(short, long, action)]
        recompress: bool,
    },

    /// 複数のビデオファイルを連結する
    Concat {
        /// 入力ビデオファイル
        #[clap(short, long, value_parser, num_args = 1..)]
        input: Vec<String>,

        /// 出力ビデオファイルパス
        #[clap(short, long, value_parser)]
        output: String,

        /// ストリームコピーの代わりにビデオを再エンコードする
        #[clap(short, long, action)]
        recompress: bool,
    },

    /// メディアファイルに関する情報を表示する
    Info {
        /// 入力メディアファイルパス
        #[clap(value_parser)]
        input: String,

        /// 詳細情報を表示する
        #[clap(short, long, action)]
        detailed: bool,
    },
    
    // 追加のコマンドはここに追加される
}
```

### コマンドインターフェース（commands.rs）

全てのコマンドのインターフェースを定義するトレイト：

```rust
/// コマンドを実装するためのトレイト
pub trait Command {
    /// コマンドの名前を取得
    fn name(&self) -> &str;
    
    /// コマンドの簡単な説明を取得
    fn description(&self) -> &str;
    
    /// コマンドの使用例を取得
    fn usage(&self) -> &str;
    
    /// 指定されたコンテキストと引数でコマンドを実行
    fn execute(&self, context: &Context, args: &[String]) -> Result<()>;
}

/// コマンドを管理するためのレジストリ
pub struct CommandRegistry {
    /// コマンド名から実装へのマップ
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    /// 新しいコマンドレジストリを作成
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }
    
    /// コマンドを登録
    pub fn register(&mut self, command: Box<dyn Command>) -> Result<()> {
        let name = command.name().to_string();
        if self.commands.contains_key(&name) {
            return Err(Error::DuplicateCommand(name));
        }
        self.commands.insert(name, command);
        Ok(())
    }
    
    /// 名前でコマンドを取得
    pub fn get(&self, name: &str) -> Result<&dyn Command> {
        self.commands.get(name)
            .map(|cmd| cmd.as_ref())
            .ok_or_else(|| Error::UnknownCommand(name.to_string()))
    }
    
    /// 登録されているすべてのコマンドを取得
    pub fn list(&self) -> Vec<&dyn Command> {
        self.commands.values()
            .map(|cmd| cmd.as_ref())
            .collect()
    }
}
```

### コマンド実装（commands.rs）

#### レンダーコマンド

```rust
/// プロジェクトレンダリングコマンド
pub struct RenderCommand;

impl RenderCommand {
    /// 新しいレンダーコマンドを作成
    pub fn new() -> Self {
        Self
    }
}

impl Command for RenderCommand {
    fn name(&self) -> &str {
        "render"
    }
    
    fn description(&self) -> &str {
        "プロジェクトを出力ファイルにレンダリングする"
    }
    
    fn usage(&self) -> &str {
        "render --project <project_file> --output <output_file> [options]"
    }
    
    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // 実装の詳細はここに記述される
        context.logger.info("レンダーコマンドを受信しました");
        context.logger.info(&format!("引数: {:?}", args));
        
        // 今のところは成功を返す - これは完全に実装されるまでのスタブです
        Ok(())
    }
}
```

#### 情報コマンド

情報コマンドはメディアファイルに関する情報を表示します：

```rust
/// メディアファイルに関する情報を表示する情報コマンド
pub struct InfoCommand;

impl InfoCommand {
    /// 新しい情報コマンドを作成
    pub fn new() -> Self {
        Self
    }
    
    /// 指定されたファイルが存在するかどうかを確認
    fn check_file_exists(&self, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(Error::CommandError(format!(
                "ファイルが存在しません: {}",
                file_path
            )));
        }
        if !path.is_file() {
            return Err(Error::CommandError(format!(
                "パスがファイルではありません: {}",
                file_path
            )));
        }
        Ok(())
    }
    
    /// 指定されたファイルからメディア情報を取得
    fn get_media_info(&self, context: &Context, file_path: &str) -> Result<MediaInfo> {
        // まずFFmpegを検出
        let ffmpeg = FFmpeg::detect()
            .map_err(|e| Error::CommandError(format!("FFmpegエラー: {e}")))?;
            
        // メディア情報を取得
        ffmpeg
            .get_media_info(file_path)
            .map_err(|e| Error::CommandError(format!("メディア情報の取得に失敗しました: {e}")))
    }
    
    /// ファイルサイズを人間が読みやすい形式にフォーマット
    fn format_file_size(&self, size_str: &str) -> Result<String> {
        let size = size_str
            .parse::<f64>()
            .map_err(|_| Error::CommandError(format!("無効なファイルサイズ: {size_str}")))?;
            
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        
        let formatted = if size >= GB {
            format!("{:.2} GB", size / GB)
        } else if size >= MB {
            format!("{:.2} MB", size / MB)
        } else if size >= KB {
            format!("{:.2} KB", size / KB)
        } else {
            format!("{} バイト", size as u64)
        };
        
        Ok(formatted)
    }
    
    /// 時間を人間が読みやすい形式にフォーマット
    fn format_duration(&self, duration_str: &str) -> Result<String> {
        let duration = duration_str
            .parse::<f64>()
            .map_err(|_| Error::CommandError(format!("無効な時間: {duration_str}")))?;
            
        let hours = (duration / 3600.0).floor() as u64;
        let minutes = ((duration % 3600.0) / 60.0).floor() as u64;
        let seconds = (duration % 60.0).floor() as u64;
        let ms = ((duration - duration.floor()) * 1000.0).round() as u64;
        
        let formatted = if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}.{ms:03}")
        } else {
            format!("{minutes:02}:{seconds:02}.{ms:03}")
        };
        
        Ok(formatted)
    }
    
    /// メディア情報を整形して表示
    fn display_media_info(&self, context: &Context, media_info: &MediaInfo) -> Result<()> {
        let format = &media_info.format;
        
        // 基本情報を表示
        context.output().info(&format!("ファイル: {}", format.filename));
        
        if let Some(size) = &format.size {
            if let Ok(formatted_size) = self.format_file_size(size) {
                context.output().info(&format!("サイズ: {formatted_size}"));
            }
        }
        
        if let Some(duration) = &format.duration {
            if let Ok(formatted_duration) = self.format_duration(duration) {
                context.output().info(&format!("長さ: {formatted_duration}"));
            }
        }
        
        context.output().info(&format!("フォーマット: {}", format.format_long_name));
        
        if let Some(bit_rate) = &format.bit_rate {
            let bit_rate_num = bit_rate.parse::<f64>().unwrap_or(0.0);
            let bit_rate_mbps = bit_rate_num / 1_000_000.0;
            context.output().info(&format!("ビットレート: {:.2} Mbps", bit_rate_mbps));
        }
        
        // ストリーム情報を表示
        context.output().info(&format!("ストリーム: {}", media_info.streams.len()));
        
        for stream in &media_info.streams {
            let codec_type = stream.codec_type.to_uppercase();
            let codec = &stream.codec_long_name;
            
            match stream.codec_type.as_str() {
                "video" => {
                    let width = stream.width.unwrap_or(0);
                    let height = stream.height.unwrap_or(0);
                    let fps = stream.frame_rate.as_deref().unwrap_or("不明");
                    
                    context.output().info(&format!(
                        "  ストリーム #{}: {} - {}, {}x{}, {} fps",
                        stream.index, codec_type, codec, width, height, fps
                    ));
                }
                "audio" => {
                    let channels = stream.channels.unwrap_or(0);
                    let sample_rate = stream.sample_rate.as_deref().unwrap_or("不明");
                    
                    context.output().info(&format!(
                        "  ストリーム #{}: {} - {}, {} Hz, {} チャンネル",
                        stream.index, codec_type, codec, sample_rate, channels
                    ));
                }
                "subtitle" => {
                    let language = stream
                        .tags
                        .as_ref()
                        .and_then(|tags| tags.get("language"))
                        .map(|s| s.as_str())
                        .unwrap_or("不明");
                        
                    context.output().info(&format!(
                        "  ストリーム #{}: {} - {}, 言語: {}",
                        stream.index, codec_type, codec, language
                    ));
                }
                _ => {
                    context.output().info(&format!(
                        "  ストリーム #{}: {} - {}",
                        stream.index, codec_type, codec
                    ));
                }
            }
        }
        
        Ok(())
    }
}

impl Command for InfoCommand {
    fn name(&self) -> &str {
        "info"
    }
    
    fn description(&self) -> &str {
        "メディアファイルに関する情報を表示する"
    }
    
    fn usage(&self) -> &str {
        "info <file_path> [--detailed]"
    }
    
    fn execute(&self, context: &Context, args: &[String]) -> Result<()> {
        // 必要な引数をチェック
        if args.is_empty() {
            return Err(Error::CommandError(
                "入力ファイルが指定されていません。使用法: info <file>".to_string(),
            ));
        }
        
        let file_path = &args[0];
        let detailed = args.len() > 1 && args[1] == "--detailed";
        
        // ファイルが存在するか確認
        self.check_file_exists(file_path)?;
        
        // メディア情報を取得
        let media_info = self.get_media_info(context, file_path)?;
        
        // 情報を表示
        self.display_media_info(context, &media_info)?;
        
        // 詳細情報が要求された場合、生のJSONを出力
        if detailed {
            context.output().info("\n詳細情報:");
            let json = serde_json::to_string_pretty(&media_info)
                .map_err(|e| Error::CommandError(format!("メディア情報のシリアル化に失敗しました: {e}")))?;
                
            context.output().info(&json);
        }
        
        Ok(())
    }
}
```

### 出力フォーマット（output.rs）

フォーマットされたコンソール出力のために：

```rust
/// ターミナル出力用の出力フォーマッタ
pub struct OutputFormatter {
    /// ANSIカラーを使用するかどうか
    use_colors: bool,
}

impl OutputFormatter {
    /// 新しい出力フォーマッタを作成
    pub fn new(use_colors: bool) -> Self {
        Self { use_colors }
    }
    
    /// 情報メッセージを出力
    pub fn info(&self, message: &str) {
        if self.use_colors {
            println!("\x1b[32m[INFO]\x1b[0m {}", message);
        } else {
            println!("[INFO] {}", message);
        }
    }
    
    /// エラーメッセージを出力
    pub fn error(&self, message: &str) {
        if self.use_colors {
            eprintln!("\x1b[31m[ERROR]\x1b[0m {}", message);
        } else {
            eprintln!("[ERROR] {}", message);
        }
    }
    
    /// 警告メッセージを出力
    pub fn warning(&self, message: &str) {
        if self.use_colors {
            println!("\x1b[33m[WARNING]\x1b[0m {}", message);
        } else {
            println!("[WARNING] {}", message);
        }
    }
    
    /// 成功メッセージを出力
    pub fn success(&self, message: &str) {
        if self.use_colors {
            println!("\x1b[32m[SUCCESS]\x1b[0m {}", message);
        } else {
            println!("[SUCCESS] {}", message);
        }
    }
}
```

### 進捗報告（output.rs）

長時間実行操作の進捗を追跡するために：

```rust
/// 操作の進捗を報告するためのインターフェース
pub trait ProgressReporter: Send {
    /// 進捗を更新
    fn update(&self, current: u64, total: u64, message: Option<&str>);
    
    /// 操作を完了としてマーク
    fn complete(&self, message: &str);
    
    /// 操作を失敗としてマーク
    fn fail(&self, message: &str);
}

/// プログレスバーを使用したコンソールベースの進捗レポーター
pub struct ConsoleProgress {
    /// プログレスバー
    progress_bar: ProgressBar,
    /// 操作の開始時間
    start_time: Instant,
}

impl ConsoleProgress {
    /// 新しいコンソール進捗レポーターを作成
    pub fn new(total: u64, title: &str) -> Self {
        let pb = ProgressBar::new(total);
        
        // プログレスバーのスタイルを設定
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        pb.set_message(title.to_string());
        
        Self {
            progress_bar: pb,
            start_time: Instant::now(),
        }
    }
}

impl ProgressReporter for ConsoleProgress {
    fn update(&self, current: u64, total: u64, message: Option<&str>) {
        if total > 0 && self.progress_bar.length() != total {
            self.progress_bar.set_length(total);
        }
        
        self.progress_bar.set_position(current);
        if let Some(msg) = message {
            self.progress_bar.set_message(msg.to_string());
        }
    }
    
    fn complete(&self, message: &str) {
        self.progress_bar.finish_with_message(message.to_string());
    }
    
    fn fail(&self, message: &str) {
        self.progress_bar.abandon_with_message(message.to_string());
    }
}
```

## エラー処理（mod.rs）

CLIモジュールは独自のエラータイプを定義します：

```rust
/// CLI操作のエラータイプ
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// コマンド実行エラー
    #[error("コマンド実行エラー: {0}")]
    CommandExecution(String),

    /// 不明なコマンド
    #[error("不明なコマンド: {0}")]
    UnknownCommand(String),

    /// 重複するコマンド登録
    #[error("重複するコマンド登録: {0}")]
    DuplicateCommand(String),

    /// 無効な引数
    #[error("無効な引数: {0}")]
    InvalidArgument(String),

    /// 不足している引数
    #[error("不足している引数: {0}")]
    MissingArgument(String),

    /// 無効なパス
    #[error("無効なパス: {0}")]
    InvalidPath(String),

    /// 無効な時間形式
    #[error("無効な時間形式: {0}")]
    InvalidTimeFormat(String),

    /// IO エラー
    #[error("IOエラー: {0}")]
    Io(#[from] std::io::Error),

    /// コアエラー
    #[error("コアエラー: {0}")]
    Core(#[from] crate::core::Error),

    /// プロジェクトエラー
    #[error("プロジェクトエラー: {0}")]
    ProjectError(String),

    /// レンダーエラー
    #[error("レンダーエラー: {0}")]
    RenderError(String),
}

/// CLI操作の結果型
pub type Result<T> = std::result::Result<T, Error>;
```

## 実装の考慮事項

### エラー処理

CLIモジュールは以下のエラー処理原則に従います：

- 構造化されたエラータイプを定義するために`thiserror`クレートを使用する
- 明確で、ユーザーフレンドリーなエラーメッセージを提供する
- エラーにコンテキスト情報を含める
- 他のモジュールからのエラー変換をサポートする
- 一貫性のあるエラー出力フォーマットを使用する
- グレースフルなアプリケーション終了を確保するために、main.rsで堅牢なエラー処理を実装する

### 進捗報告

進捗報告システムは以下の原則に従います：

- トレイトを通じて進捗報告を抽象化する
- 既知および未知の総作業量を持つ操作をサポートする
- 意味のある時間見積もりを提供する
- ネストされた進捗報告をサポートする
- 進行中の操作のキャンセルをサポートする

### コマンド構造

CLIモジュールのコマンドは以下の原則に従います：

- コマンド間で一貫したパラメータの命名
- オプションパラメータの合理的なデフォルト値
- 包括的なヘルプテキスト
- 明確なエラーメッセージによる入力検証
- 破壊的な操作の確認

## 他のモジュールとの統合

### コアモジュール統合

- 設定とロギングのためにコアモジュールを使用
- コマンドの実行コンテキストを作成
- コアの操作からのエラーを伝播

### プロジェクトモジュール統合

- プロジェクト管理のためのコマンドを提供
- プロジェクトの読み込みと保存をサポート
- タイムライン編集とレンダリングを可能に

### FFmpegモジュール統合

- メディア操作のためのFFmpegモジュールとの深い統合
- FFmpegインストールを検出し検証するために`FFmpeg::detect()`を使用
- 包括的なメディアファイルの詳細を取得するために`get_media_info()`を活用
- 適切なエラー処理でメディア情報をフォーマットし表示
- ビデオ、オーディオ、字幕ストリームに関する詳細情報を提供
- FFmpegの可用性と実行エラーをうまく処理

### オーディオモジュール統合

- オーディオ処理のためのコマンドを提供
- ボリューム調整と抽出をサポート
- オーディオの置換とエフェクトを可能に

### 字幕モジュール統合

- 字幕処理のためのコマンドを提供
- 字幕編集とフォーマットをサポート
- 字幕の抽出と注入を可能に

## 外部ライブラリとの統合

### mime_guess統合

CLIモジュールはファイルタイプを検出するために`mime_guess`クレートを使用します（特にInfoCommandで）：

- ファイル拡張子に基づいてファイルのMIMEタイプを検出できる
- ユーザーフレンドリーなファイルタイプ情報を提供
- メディアファイル識別機能を強化
- FFmpeg処理に有効なメディアファイルかどうかを判断するために使用

### chrono統合

モジュールは日付と時刻の処理に`chrono`クレートを使用します：

- ファイルのタイムスタンプを人間が読みやすい形式でフォーマット（UTCベース）
- ファイル統計のための日付計算を提供
- ログと出力のための正確な時間フォーマットを可能に

### serde_json統合

メディアファイル情報の処理用：

- FFmpegのJSON出力を構造化データに解析
- メディア情報への型安全なアクセスを可能に
- 詳細なメディアファイル分析をサポート

## 実装状況の更新（2024年）

### 現在の実装状況：進行中（〜60%）

CLIモジュールはコア機能が整備され、現在活発に開発が行われています。メインアプリケーション構造とコマンド解析は完了しており、いくつかの主要コマンドが実装されています。

| コンポーネント | 状況 | 実装レベル | 備考 |
|------------|------|-----------|------|
| アプリ構造 | ✅ 完了 | 100% | アプリケーションエントリーポイントとコマンドディスパッチャー |
| コマンドライン解析 | ✅ 完了 | 100% | clapによるコマンドライン引数解析 |
| コマンドレジストリ | ✅ 完了 | 100% | コマンド登録と検索 |
| InfoCommand | ✅ 完了 | 100% | メディアファイル情報を表示 |
| TrimCommand | 🔄 進行中 | 80% | 基本的なトリミング機能が実装済み |
| ConcatCommand | 🔄 進行中 | 50% | 基本的な連結が実装済み |
| その他のコマンド | 📝 計画段階 | 0% | まだ実装されていない |

### 実装済みの主要機能

1. **アプリケーション構造**
   - コマンドライン引数解析
   - コマンドレジストリ
   - 設定とロギングを含む実行コンテキスト

2. **コアコマンド**
   - 情報コマンド
   - 基本的なトリミング機能
   - 初期連結サポート

### 将来の開発計画

1. **コアコマンドの完成**
   - トリムと連結コマンドの実装を完了
   - ✅ FFmpegメディア詳細情報付きの情報コマンドの強化（完了）
   - 堅牢なエラー処理と検証を追加
   - 長時間実行操作の進捗報告を実装

2. **FFmpeg統合の強化**
   - ✅ 詳細なメディアファイル分析を追加（完了）
   - FFmpeg検出と検証を改善
   - より多くのエンコーディングオプションのサポートを追加
   - FFmpegエラーをより適切に処理

3. **高度なコマンドの実装**
   - オーディオ抽出と置換のサポートを追加
   - 字幕処理を実装
   - バッチ操作のサポート
   
4. **ユーザーエクスペリエンスの改善**
   - より詳細な進捗報告を追加
   - よりよいトラブルシューティングのためにエラーメッセージを強化
   - 複雑な操作のための対話モードを追加

CLIモジュールは、使いやすさ、一貫性、およびアプリケーションの拡大する機能セットとの統合に焦点を当てて、edvアプリケーションの主要なインターフェースとして進化し続けます。