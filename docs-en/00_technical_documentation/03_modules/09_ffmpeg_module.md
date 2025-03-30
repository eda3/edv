# edv - FFmpeg Module Implementation

このドキュメントでは、edvアプリケーションのFFmpegモジュールの実装の詳細について説明します。

## 概要

FFmpegモジュールは、外部のFFmpegコマンドラインツールとの統合を提供し、動画・音声処理の中核機能を実現します。このモジュールは、FFmpegの検出、バージョン検証、コマンド構築、実行、および出力解析を担当します。

## 構造

```
src/ffmpeg/
├── mod.rs       # モジュール定義、FFmpeg構造体、MediaInfo構造体
├── command.rs   # FFmpegコマンド構築・実行ユーティリティ
└── error.rs     # エラー型定義
```

## 主要コンポーネント

### FFmpeg (mod.rs)

FFmpeg実行環境との連携を担当する構造体:

```rust
/// FFmpeg executable wrapper
pub struct FFmpeg {
    /// Path to the FFmpeg executable
    path: PathBuf,
    /// FFmpeg version
    version: Version,
}

impl FFmpeg {
    /// Minimum supported FFmpeg version
    const MIN_VERSION: Version = Version::new(4, 3, 0);

    /// Creates a new FFmpeg instance.
    pub fn new(path: PathBuf, version: Version) -> Self {
        Self { path, version }
    }
    
    /// Detects FFmpeg in the system PATH.
    pub fn detect() -> Result<Self> {
        // Try to find ffmpeg in PATH
        let ffmpeg_path = which::which("ffmpeg")
            .map_err(|_| Error::NotFound)?;
            
        Self::detect_at_path(ffmpeg_path)
    }
    
    /// Detects FFmpeg at the specified path.
    pub fn detect_at_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Execute ffmpeg -version to get version info
        let output = Command::new(&path)
            .arg("-version")
            .output()?;
            
        if !output.status.success() {
            return Err(Error::ExecutionError(
                format!("FFmpeg validation failed with status: {}", output.status)
            ));
        }
        
        // Parse version from output
        let version_output = String::from_utf8_lossy(&output.stdout);
        let version = Self::parse_version_from_output(&version_output)?;
        
        // Create and validate the instance
        let ffmpeg = Self::new(path, version);
        ffmpeg.validate()?;
        
        Ok(ffmpeg)
    }
    
    /// Validates that the FFmpeg installation is compatible.
    pub fn validate(&self) -> Result<()> {
        if self.version < Self::MIN_VERSION {
            return Err(Error::UnsupportedVersion {
                actual: self.version.clone(),
                required: Self::MIN_VERSION,
            });
        }
        Ok(())
    }
    
    /// Gets detailed information about a media file.
    pub fn get_media_info<P: AsRef<Path>>(&self, file_path: P) -> Result<MediaInfo> {
        let path = file_path.as_ref();
        
        if !path.exists() {
            return Err(Error::InvalidPath(format!(
                "File not found: {}",
                path.display()
            )));
        }
        
        // Use ffprobe to get media information
        let output = std::process::Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(path)
            .output()
            .map_err(|e| Error::ExecutionError(format!("Failed to execute ffprobe: {e}")))?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::ProcessTerminated {
                exit_code: output.status.code(),
                message: format!("ffprobe process failed: {stderr}"),
            });
        }
        
        // Parse the JSON output
        let output_str = String::from_utf8_lossy(&output.stdout);
        let media_info: MediaInfo = serde_json::from_str(&output_str)
            .map_err(|e| Error::OutputParseError(format!("Failed to parse ffprobe output: {e}")))?;
        
        Ok(media_info)
    }
}
```

### MediaInfo (mod.rs)

メディアファイルの情報を格納するための構造体:

```rust
/// Represents media format information.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FormatInfo {
    /// The filename.
    pub filename: String,
    /// The number of streams.
    #[serde(default)]
    pub nb_streams: i32,
    /// The number of programs.
    #[serde(default)]
    pub nb_programs: i32,
    /// The format name.
    #[serde(default)]
    pub format_name: String,
    /// The format long name.
    #[serde(rename = "format_long_name", default)]
    pub format_long_name: String,
    /// The start time in seconds.
    #[serde(default)]
    pub start_time: Option<String>,
    /// The duration in seconds.
    #[serde(default)]
    pub duration: Option<String>,
    /// The size in bytes.
    #[serde(default)]
    pub size: Option<String>,
    /// The bit rate in bits per second.
    #[serde(default)]
    pub bit_rate: Option<String>,
    /// The probe score (higher is better).
    #[serde(default)]
    pub probe_score: i32,
    /// Additional tags.
    #[serde(default)]
    pub tags: Option<std::collections::HashMap<String, String>>,
}

/// Represents a media stream (video, audio, subtitle, etc.).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StreamInfo {
    /// The index of the stream.
    pub index: i32,
    /// The codec type (video, audio, subtitle, etc.).
    #[serde(rename = "codec_type")]
    pub codec_type: String,
    /// The codec name.
    #[serde(rename = "codec_name", default)]
    pub codec_name: String,
    /// The codec long name.
    #[serde(rename = "codec_long_name", default)]
    pub codec_long_name: String,
    /// The width (for video streams).
    #[serde(default)]
    pub width: Option<i32>,
    /// The height (for video streams).
    #[serde(default)]
    pub height: Option<i32>,
    /// The pixel format (for video streams).
    #[serde(rename = "pix_fmt", default)]
    pub pixel_format: Option<String>,
    /// The frame rate (for video streams).
    #[serde(rename = "r_frame_rate", default)]
    pub frame_rate: Option<String>,
    /// The sample rate (for audio streams).
    #[serde(rename = "sample_rate", default)]
    pub sample_rate: Option<String>,
    /// The number of channels (for audio streams).
    #[serde(default)]
    pub channels: Option<i32>,
    /// The channel layout (for audio streams).
    #[serde(rename = "channel_layout", default)]
    pub channel_layout: Option<String>,
    /// The bit rate (for audio/video streams).
    #[serde(default)]
    pub bit_rate: Option<String>,
    /// Additional tags.
    #[serde(default)]
    pub tags: Option<std::collections::HashMap<String, String>>,
}

/// Represents comprehensive media information.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MediaInfo {
    /// Information about the format (container).
    pub format: FormatInfo,
    /// Information about the streams (video, audio, subtitle, etc.).
    pub streams: Vec<StreamInfo>,
}

impl MediaInfo {
    /// Gets the video streams.
    pub fn video_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|stream| stream.codec_type == "video")
            .collect()
    }
    
    /// Gets the audio streams.
    pub fn audio_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|stream| stream.codec_type == "audio")
            .collect()
    }
    
    /// Gets the subtitle streams.
    pub fn subtitle_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|stream| stream.codec_type == "subtitle")
            .collect()
    }
    
    /// Gets the duration in seconds.
    pub fn duration_seconds(&self) -> Option<f64> {
        self.format
            .duration
            .as_ref()
            .and_then(|s| s.parse::<f64>().ok())
    }
    
    /// Gets the bit rate in bits per second.
    pub fn bit_rate(&self) -> Option<u64> {
        self.format
            .bit_rate
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
    }
}
```

### FFmpegCommand (command.rs)

FFmpegコマンドの構築と実行を抽象化する構造体:

```rust
/// FFmpeg command builder.
pub struct FFmpegCommand {
    /// The FFmpeg executable path
    ffmpeg_path: PathBuf,
    /// Command arguments
    args: Vec<String>,
    /// Input file paths
    inputs: Vec<PathBuf>,
    /// Output file path
    output: Option<PathBuf>,
}

impl FFmpegCommand {
    /// Creates a new FFmpeg command.
    pub fn new(ffmpeg_path: PathBuf) -> Self {
        Self {
            ffmpeg_path,
            args: Vec::new(),
            inputs: Vec::new(),
            output: None,
        }
    }
    
    /// Adds an input file to the command.
    pub fn add_input<P: AsRef<Path>>(mut self, input: P) -> Self {
        let input_path = input.as_ref().to_path_buf();
        self.args.push("-i".to_string());
        self.args.push(input_path.to_string_lossy().to_string());
        self.inputs.push(input_path);
        self
    }
    
    /// Sets the output file for the command.
    pub fn set_output<P: AsRef<Path>>(mut self, output: P) -> Self {
        let output_path = output.as_ref().to_path_buf();
        self.output = Some(output_path.clone());
        self.args.push(output_path.to_string_lossy().to_string());
        self
    }
    
    /// Adds a custom argument to the command.
    pub fn add_arg<S: AsRef<str>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_string());
        self
    }
    
    /// Executes the command.
    pub fn execute(&self) -> Result<(), Error> {
        // Validate that we have at least one input
        if self.inputs.is_empty() {
            return Err(Error::MissingInput);
        }
        
        // Validate that we have an output
        if self.output.is_none() {
            return Err(Error::MissingOutput);
        }
        
        // Build and execute the command
        let mut command = Command::new(&self.ffmpeg_path);
        command.args(&self.args);
        
        // Execute the command
        let output = command.output()
            .map_err(|e| Error::ExecutionError(e.to_string()))?;
            
        // Check if the command was successful
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::ProcessTerminated {
                exit_code: output.status.code(),
                message: format!("FFmpeg process failed: {}", stderr),
            });
        }
        
        Ok(())
    }
}
```

### Error (error.rs)

FFmpegモジュール固有のエラー型:

```rust
/// FFmpeg module errors.
#[derive(Debug, Error)]
pub enum Error {
    /// FFmpeg not found
    #[error("FFmpeg not found")]
    NotFound,
    
    /// Unsupported FFmpeg version
    #[error("Unsupported FFmpeg version: {actual}, required: {required}")]
    UnsupportedVersion {
        /// Actual FFmpeg version
        actual: Version,
        /// Required FFmpeg version
        required: Version,
    },
    
    /// Error executing FFmpeg
    #[error("Error executing FFmpeg: {0}")]
    ExecutionError(String),
    
    /// Error parsing FFmpeg output
    #[error("Error parsing FFmpeg output: {0}")]
    OutputParseError(String),
    
    /// Missing input file
    #[error("Missing input file")]
    MissingInput,
    
    /// Missing output file
    #[error("Missing output file")]
    MissingOutput,
    
    /// FFmpeg process terminated with error
    #[error("FFmpeg process terminated: {message}")]
    ProcessTerminated {
        /// Process exit code
        exit_code: Option<i32>,
        /// Error message
        message: String,
    },
    
    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## メディア情報取得機能

FFmpegモジュールの主要な機能の一つが、`get_media_info`メソッドを使用したメディアファイルの詳細情報取得です：

1. **ffprobeコマンドの実行**: FFmpegの一部であるffprobeを使用してメディアファイルを分析
2. **JSON形式の出力取得**: `-print_format json`オプションを使用して構造化された情報を取得
3. **構造体へのデシリアライズ**: serde_jsonを使用してJSONをMediaInfo構造体に変換
4. **情報の整理と分析**: ストリーム別（動画、音声、字幕）の情報アクセスメソッドを提供
5. **ユーティリティメソッド**: 長さやビットレートなどの取得を簡単にするヘルパーメソッド

## InfoCommand との統合

CLIモジュールのInfoCommandは、FFmpegモジュールを使用して以下の情報を表示します：

1. **フォーマット情報**: コンテナ形式、長さ、ビットレート
2. **動画ストリーム情報**: コーデック、解像度、フレームレート
3. **音声ストリーム情報**: コーデック、サンプルレート、チャンネル数
4. **字幕ストリーム情報**: コーデック、言語
5. **詳細メタデータ**: `--detailed`フラグが指定された場合の追加情報

## エラー処理

FFmpegモジュールは堅牢なエラー処理を実装しています：

1. **FFmpeg存在確認**: システムにFFmpegがインストールされているか確認
2. **バージョン検証**: サポートされているバージョンであることを確認
3. **ファイル存在確認**: 指定されたメディアファイルの存在を確認
4. **プロセス実行エラー**: FFmpegコマンド実行時のエラーをキャプチャ
5. **出力解析エラー**: JSON解析時のエラーを詳細に報告

## 実装状況（2024年3月）

FFmpegモジュールは以下の状況で開発中です：

| コンポーネント | 状態 | 備考 |
|--------------|------|------|
| FFmpeg検出 | ✅ 完了 | システムパスとカスタムパスをサポート |
| バージョン検証 | ✅ 完了 | 最小バージョン要件の検証 |
| メディア情報取得 | ✅ 完了 | 詳細なメディア情報取得とフォーマット |
| 基本コマンド構築 | ✅ 完了 | 入力、出力、引数の設定 |
| コマンド実行 | ✅ 完了 | 非同期実行と結果処理 |
| エラー処理 | ✅ 完了 | 包括的なエラー型と処理 |
| トリミング操作 | 🔄 進行中 | 開始点と終了点の指定による動画トリミング |
| 連結操作 | 🔄 進行中 | 複数動画ファイルの連結 |
| フォーマット変換 | 🔶 計画中 | 動画フォーマット変換機能 |
| ストリーム抽出 | 🔶 計画中 | 音声/字幕ストリームの抽出 |
| フィルタ適用 | 🔶 計画中 | 複雑なFFmpegフィルタの適用 |

## 今後の開発計画

FFmpegモジュールの今後の開発計画は以下の通りです：

1. **トリミングと連結のサポート強化**
   - 正確なフレーム単位のカットポイント指定
   - 再エンコード/ストリームコピーオプション
   - 複数フォーマットのサポート改善

2. **動画処理フィルタ**
   - クロップ、リサイズ、回転などの基本変換
   - カラー調整、ノイズ除去などの高度効果
   - GPU加速のサポート

3. **エンコード設定**
   - プリセットベースの出力設定
   - カスタムパラメータのサポート強化
   - 品質/サイズ最適化オプション

4. **プログレス報告システム**
   - FFmpeg処理の進捗状況リアルタイム表示
   - キャンセル可能な長時間処理
   - 詳細ログオプション

5. **バッチ処理**
   - 複数ファイルの一括処理
   - テンプレート基づくファイル名生成
   - ジョブキューシステム

これらの機能強化により、FFmpegモジュールはedvアプリケーションの中核的な動画処理エンジンとしての役割を果たします。 