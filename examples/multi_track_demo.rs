/// マルチトラック編集のサンプルプロジェクト
///
/// このサンプルでは、複数のビデオトラックとオーディオトラックを含むプロジェクトを作成し、
/// それらを合成して最終的な動画を出力します。
///
/// # 実行方法
///
/// ```bash
/// # WSL環境で実行する場合は、TMPDIR環境変数を設定してから実行してください
/// # まず、一時ディレクトリを作成
/// mkdir -p output/temp
/// chmod -R 1777 output
///
/// # 次に、環境変数を設定して実行
/// TMPDIR=$(pwd)/output/temp cargo run --example multi_track_demo
/// ```
///
/// # TODO
///
/// * WSL環境で実行時の一時ディレクトリ作成の問題を修正する
/// * より柔軟なメディアファイルパスの指定方法を導入する
use std::path::{Path, PathBuf};

use edv::ffmpeg::FFmpeg;
use edv::project::rendering::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
use edv::project::timeline::{Clip, TrackKind};
use edv::project::{AssetId, AssetMetadata, AssetReference, ClipId, Project};
use edv::utility::time::{Duration, TimePosition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // FFmpegを初期化
    let ffmpeg = FFmpeg::detect()?;
    println!("FFmpeg detected: {}", ffmpeg.path().display());
    println!("FFmpeg version: {}", ffmpeg.version());

    // 出力ディレクトリを作成
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // プロジェクトを作成
    let mut project = Project::new("マルチトラックサンプル");

    // 入力ファイルのパスを設定
    let input_file = PathBuf::from("test_media/sozai.mp4");
    if !input_file.exists() {
        return Err(format!("Input file not found: {}", input_file.display()).into());
    }

    // 入力ファイルのメディア情報を取得
    let media_info = ffmpeg.get_media_info(&input_file)?;

    // メディア情報から基本データを取得
    let main_video_duration = media_info.duration_seconds().unwrap_or(10.0);

    // メディア情報からビデオ解像度を取得
    let video_dimensions = if let Some(video_stream) = media_info.video_streams().first() {
        (
            video_stream.width.unwrap_or(1280) as u32,
            video_stream.height.unwrap_or(720) as u32,
        )
    } else {
        (1280, 720) // デフォルト値
    };

    // アセットメタデータを作成
    let mut asset_metadata = AssetMetadata {
        duration: Some(Duration::from_seconds(main_video_duration)),
        dimensions: Some(video_dimensions),
        asset_type: "video".to_string(),
        extra: std::collections::HashMap::new(),
    };

    // アセットを追加
    let asset_id = project.add_asset(input_file.clone(), asset_metadata.clone());

    // 2つ目のアセットとして同じファイルを追加 (実際のプロジェクトでは異なるファイルを使用することが多い)
    asset_metadata
        .extra
        .insert("purpose".to_string(), "overlay".to_string());
    let overlay_asset_id = project.add_asset(input_file.clone(), asset_metadata);

    // タイムラインにトラックを追加
    let main_video_track_id = project.timeline.add_track(TrackKind::Video);
    let overlay_video_track_id = project.timeline.add_track(TrackKind::Video);
    let audio_track_id = project.timeline.add_track(TrackKind::Audio);

    // トラック名を設定
    if let Some(track) = project.timeline.get_track_mut(main_video_track_id) {
        track.set_name("メインビデオ");
    }

    if let Some(track) = project.timeline.get_track_mut(overlay_video_track_id) {
        track.set_name("オーバーレイビデオ");
    }

    if let Some(track) = project.timeline.get_track_mut(audio_track_id) {
        track.set_name("オーディオ");
    }

    // クリップを作成してメインビデオトラックに追加
    let main_clip = Clip::new(
        ClipId::new(),
        asset_id,
        TimePosition::from_seconds(0.0),
        Duration::from_seconds(main_video_duration),
        TimePosition::from_seconds(0.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project.timeline.add_clip(main_video_track_id, main_clip)?;

    // オーバーレイビデオクリップを作成（元の動画の一部を切り取って別のタイミングで表示）
    let overlay_clip = Clip::new(
        ClipId::new(),
        overlay_asset_id,
        TimePosition::from_seconds(2.0), // 2秒後から開始
        Duration::from_seconds(3.0),     // 3秒間表示
        TimePosition::from_seconds(1.0), // 元動画の1秒目から
        TimePosition::from_seconds(4.0), // 元動画の4秒目まで
    );

    project
        .timeline
        .add_clip(overlay_video_track_id, overlay_clip)?;

    // オーディオクリップを追加（元の動画のオーディオとして）
    let audio_clip = Clip::new(
        ClipId::new(),
        asset_id, // 同じアセットのオーディオを使用
        TimePosition::from_seconds(0.0),
        Duration::from_seconds(main_video_duration),
        TimePosition::from_seconds(0.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project.timeline.add_clip(audio_track_id, audio_clip)?;

    // マルチトラックの関係を設定（オーバーレイは半透明で表示）
    // 注：現在のAPIではset_track_opacityメソッドがないため、キーフレームで代用が必要です
    // このサンプルではコメントアウトします
    /*
    let multi_track_manager = project.timeline.multi_track_manager_mut();
    multi_track_manager.set_track_opacity(overlay_video_track_id, 0.5)?;
    */

    // 代わりにキーフレームで不透明度を設定
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(0.0),
        0.5, // 50%の不透明度
        edv::project::timeline::keyframes::EasingFunction::Linear,
    )?;

    // 出力パスが存在することを確認
    let output_path = output_dir.join("multi_track_output.mp4");

    // レンダリング設定を作成
    let render_config = RenderConfig::new(output_path.clone())
        .with_resolution(1280, 720)
        .with_frame_rate(30.0)
        .with_video_settings(VideoCodec::H264, 85)
        .with_audio_settings(AudioCodec::AAC, 80)
        .with_format(OutputFormat::MP4);

    // レンダリングを実行
    println!("Rendering project to: {}", output_path.display());
    let result = project.render_with_config(render_config)?;

    println!("Successfully rendered to: {}", result.output_path.display());
    println!("Duration: {} seconds", result.duration.as_seconds());
    Ok(())
}
