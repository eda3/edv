# edv - FFmpegモジュール実装

このドキュメントでは、edvアプリケーションのFFmpeg統合モジュールの詳細な実装ガイドラインを提供します。

**最終更新日: 2025年4月1日**

## 最近の更新

- FFmpegCommandクラスのメモリ使用効率を最適化（ベクトルの初期容量設定）
- 不要なクローンと文字列変換を削除
- executeメソッドのパフォーマンス向上とエラーハンドリングの改善
- バッファリング処理の効率化

## 概要

FFmpegモジュールは、外部のFFmpegライブラリとの統合を担当し、ビデオ編集の核となる機能を提供します。このモジュールはFFmpegの検出、バージョン検証、コマンド構築、実行、結果の解析などの機能を抽象化し、edvアプリケーションの他のコンポーネントに対して一貫したインターフェースを提供します。

## 構造

```
src/ffmpeg/
├── mod.rs       # モジュールのエクスポート、コア機能、メディア情報取得
├── command.rs   # FFmpegコマンド構築ユーティリティ
└── error.rs     # エラー型と結果型の定義
```

## 主要コンポーネント

### FFmpeg（mod.rs）

FFmpegバイナリとのインタラクションを管理する中心的なクラス：

```rust
/// Represents a detected `FFmpeg` installation.
#[derive(Debug, Clone)]
pub struct FFmpeg {
    /// The path to the `FFmpeg` executable.
    path: PathBuf,
    /// The `FFmpeg` version.
    version: Version,
}

impl FFmpeg {
    /// The minimum supported `FFmpeg` version.
    pub const MIN_VERSION: Version = Version {
        major: 4,
        minor: 0,
        patch: 0,
    };
    
    /// Creates a new `FFmpeg` instance.
    pub fn new(path: PathBuf, version: Version) -> Self;
    
    /// Gets the path to the `FFmpeg` executable.
    pub fn path(&self) -> &Path;
    
    /// Gets the `FFmpeg` version.
    pub fn version(&self) -> &Version;
    
    /// Detects FFmpeg in the system.
    pub fn detect() -> Result<Self>;
    
    /// Detects FFmpeg at the specified path.
    pub fn detect_at_path<P: AsRef<Path>>(path: P) -> Result<Self>;
    
    /// Creates a new command builder.
    pub fn command(&self) -> crate::ffmpeg::command::FFmpegCommand;
    
    /// Gets media information for a file.
    pub fn get_media_info<P: AsRef<Path>>(&self, file_path: P) -> Result<MediaInfo>;
    
    /// Validates that the FFmpeg version meets requirements.
    pub fn validate(&self) -> Result<()>;
}
```

### Version（mod.rs）

FFmpegのバージョン情報を表現するクラス：

```rust
/// Represents an `FFmpeg` version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Patch version number.
    pub patch: u32,
}

impl Version {
    /// Creates a new version from components.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self;
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl FromStr for Version {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err>;
}
```

### MediaInfo（mod.rs）

メディアファイルの情報を表現するクラス：

```rust
pub struct MediaInfo {
    /// Information about the format (container).
    pub format: FormatInfo,
    /// Information about the streams (video, audio, subtitle, etc.).
    pub streams: Vec<StreamInfo>,
}

impl MediaInfo {
    /// Gets all video streams.
    pub fn video_streams(&self) -> Vec<&StreamInfo>;
    
    /// Gets all audio streams.
    pub fn audio_streams(&self) -> Vec<&StreamInfo>;
    
    /// Gets all subtitle streams.
    pub fn subtitle_streams(&self) -> Vec<&StreamInfo>;
    
    /// Gets the duration in seconds.
    pub fn duration_seconds(&self) -> Option<f64>;
    
    /// Gets the bit rate.
    pub fn bit_rate(&self) -> Option<u64>;
}
```

### FFmpegCommand（command.rs）

FFmpegコマンドを構築・実行するためのビルダーパターンを実装：

```rust
/// Represents an `FFmpeg` command.
#[derive(Debug, Clone)]
pub struct FFmpegCommand<'a> {
    /// The `FFmpeg` instance to use.
    ffmpeg: &'a FFmpeg,
    /// Input options to apply before specifying inputs.
    input_options: Vec<String>,
    /// Input files for the command.
    inputs: Vec<PathBuf>,
    /// Filter complex to apply (if any).
    filter_complex: Option<String>,
    /// Output options to apply before specifying output.
    output_options: Vec<String>,
    /// Output file for the command.
    output: Option<PathBuf>,
    /// Whether to overwrite output file if it exists.
    overwrite: bool,
}

impl<'a> FFmpegCommand<'a> {
    /// Creates a new `FFmpeg` command with optimized initial vector capacities.
    pub fn new(ffmpeg: &'a FFmpeg) -> Self;
    
    /// Adds input options to be applied before an input file.
    /// Reserves capacity based on estimated size to reduce reallocations.
    pub fn input_options<S: AsRef<str>, I: IntoIterator<Item = S>>(
        &mut self,
        options: I,
    ) -> &mut Self;
    
    /// Adds an input file to the command.
    pub fn input<P: AsRef<Path>>(&mut self, input: P) -> &mut Self;
    
    /// Sets a filter complex for the command.
    pub fn filter_complex<S: AsRef<str>>(&mut self, filter: S) -> &mut Self;
    
    /// Adds output options to be applied before the output file.
    pub fn output_options<S: AsRef<str>, I: IntoIterator<Item = S>>(
        &mut self,
        options: I,
    ) -> &mut Self;
    
    /// Sets the output file for the command.
    pub fn set_output<P: AsRef<Path>>(&mut self, output: P) -> &mut Self;
    
    /// Sets whether to overwrite the output file if it exists.
    pub fn overwrite(&mut self, overwrite: bool) -> &mut Self;
    
    /// Executes the `FFmpeg` command with optimized validation and error handling.
    /// Includes improved buffering for standard output and error streams.
    pub fn execute(&self) -> Result<()>;
    
    /// Executes the `FFmpeg` command with progress reporting.
    /// Optimized for efficient process management and error detection.
    pub fn execute_with_progress<F>(&self, progress_callback: F) -> Result<()>
    where
        F: FnMut(&str);
}
```

### エラー処理（error.rs）

FFmpegモジュール固有のエラータイプを定義：

```rust
/// Error type for `FFmpeg` operations.
#[derive(Error, Debug)]
pub enum Error {
    /// `FFmpeg` executable not found.
    #[error("FFmpeg executable not found")]
    NotFound,
    
    /// `FFmpeg` process timeout.
    #[error("FFmpeg process timed out")]
    Timeout,
    
    /// `FFmpeg` error output.
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),
    
    /// IO error.
    #[error("IO error: {0}")]
    IoError(String),
    
    /// Error parsing `FFmpeg` output.
    #[error("Error parsing FFmpeg output: {0}")]
    ParseError(String),

    /// Missing output file specification.
    #[error("No output file specified")]
    MissingOutput,

    /// Missing input file specification.
    #[error("No input files specified")]
    MissingInput,

    /// Process execution error.
    #[error("Process error: {0}")]
    ProcessError(String),
}

/// Result type for `FFmpeg` operations.
pub type Result<T> = std::result::Result<T, Error>;
```

## 使用例

### FFmpegの検出と初期化

```rust
use edv::ffmpeg::FFmpeg;

fn initialize_ffmpeg() -> Result<FFmpeg> {
    // システム内でFFmpegを自動検出
    let ffmpeg = FFmpeg::detect()?;
    
    // FFmpegのバージョンを検証
    ffmpeg.validate()?;
    
    println!("FFmpeg found at: {}", ffmpeg.path().display());
    println!("FFmpeg version: {}", ffmpeg.version());
    
    Ok(ffmpeg)
}
```

### メディア情報の取得

```rust
use edv::ffmpeg::FFmpeg;
use std::path::Path;

fn print_media_info(ffmpeg: &FFmpeg, file_path: &Path) -> Result<()> {
    let info = ffmpeg.get_media_info(file_path)?;
    
    println!("File: {}", file_path.display());
    println!("Duration: {:?} seconds", info.duration_seconds());
    println!("Format: {}", info.format.format_long_name);
    
    println!("Video streams:");
    for stream in info.video_streams() {
        println!("  - {}x{} @ {}", 
            stream.width.unwrap_or(0), 
            stream.height.unwrap_or(0),
            stream.frame_rate.as_deref().unwrap_or("unknown"));
    }
    
    println!("Audio streams:");
    for stream in info.audio_streams() {
        println!("  - {} channels @ {} Hz", 
            stream.channels.unwrap_or(0),
            stream.sample_rate.as_deref().unwrap_or("unknown"));
    }
    
    Ok(())
}
```

### コマンドの構築と実行

```rust
use edv::ffmpeg::FFmpeg;
use std::path::Path;

fn trim_video(
    ffmpeg: &FFmpeg,
    input: &Path,
    output: &Path,
    start: &str,
    end: &str
) -> Result<()> {
    ffmpeg.command()
        .add_input_option("-ss", start)
        .add_input_option("-to", end)
        .input(input)
        .add_output_option("-c", "copy")
        .set_output(output)
        .overwrite(true)
        .execute()
}
```

## 設計上の考慮事項

1. **自動検出**: FFmpegモジュールは、PATH環境変数や一般的なインストール場所でFFmpegを自動的に検出できます。これにより、ユーザーは手動でパスを指定する必要がなくなります。

2. **バージョン検証**: サポート対象の最小バージョン（4.0.0）を定義し、検出されたFFmpegが互換性があるかどうかを確認します。

3. **効率的なコマンド構築**: ビルダーパターンを使用してFFmpegコマンドを構築することで、メソッドチェーンを通じた流暢なAPIを提供します。ベクトルの初期容量設定により、メモリの再割り当てを最小限に抑えます。

4. **進捗報告**: 長時間実行されるFFmpeg操作中の進捗状況をリアルタイムで報告する機能を提供します。

5. **堅牢なエラー処理**: FFmpeg固有のエラー（実行可能ファイルが見つからない、タイムアウト、FFmpegエラー出力など）を適切に処理し、詳細なエラーメッセージを提供します。

6. **最適化されたプロセス管理**: 標準出力と標準エラー出力の効率的なバッファリングにより、プロセス実行のオーバーヘッドを削減します。

## 制限事項と将来の拡張可能性

1. **FFmpeg依存**: このモジュールは外部FFmpegバイナリに依存しています。将来的には、FFmpegライブラリの直接バインディングを提供することで、この依存関係を削減または排除することを検討する価値があります。

2. **進捗解析の堅牢性**: FFmpeg出力形式の変更に対応するために、進捗解析をより堅牢にする余地があります。

3. **並列処理**: 現在、一度に一つのFFmpegプロセスしか管理できません。将来的には、複数のFFmpeg操作を並列して実行する機能を追加する価値があるでしょう。

4. **ハードウェアアクセラレーション**: ハードウェアアクセラレーションオプションの検出と使用のためのユーティリティを追加することで、パフォーマンスを向上させることができます。

5. **コマンドのキャッシュと再利用**: 類似コマンドのキャッシュと再利用メカニズムを実装し、同じ設定での繰り返し実行時の効率を向上させることができます。 