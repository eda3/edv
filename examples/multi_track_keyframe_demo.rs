/// キーフレームアニメーション付きマルチトラック編集のサンプルプロジェクト
///
/// このサンプルでは、複数のビデオトラックとオーディオトラックを含むプロジェクトを作成し、
/// キーフレームアニメーションを使って不透明度や位置を時間とともに変化させます。
use std::path::{Path, PathBuf};

use edv::ffmpeg::FFmpeg;
use edv::project::rendering::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
use edv::project::timeline::keyframes::EasingFunction;
use edv::project::timeline::{Clip, TrackKind};
use edv::project::{AssetId, AssetMetadata, AssetReference, ClipId, Project};
use edv::utility::time::{Duration, TimePosition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // FFmpegを初期化
    let ffmpeg = FFmpeg::detect()?;
    println!("FFmpeg detected: {}", ffmpeg.path().display());
    println!("FFmpeg version: {}", ffmpeg.version());

    // プロジェクトを作成
    let mut project = Project::new("キーフレームマルチトラックサンプル");

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
        track.set_name("キーフレームアニメーション付きオーバーレイ");
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

    // オーバーレイビデオクリップを作成（5秒間表示）
    let overlay_clip = Clip::new(
        ClipId::new(),
        overlay_asset_id,
        TimePosition::from_seconds(1.0), // 1秒後から開始
        Duration::from_seconds(5.0),     // 5秒間表示
        TimePosition::from_seconds(0.0), // 元動画の最初から
        TimePosition::from_seconds(5.0), // 元動画の5秒目まで
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

    // オーバーレイのキーフレームアニメーションを追加

    // 不透明度のキーフレームアニメーション
    // 開始時: 0% (不可視) -> 1秒後: 100% (完全に表示) -> 4秒後: 100% -> 5秒後: 0% (フェードアウト)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(1.0), // 開始時点
        0.0,                             // 開始時の不透明度 (0%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(2.0), // 1秒後
        1.0,                             // 不透明度 100%
        EasingFunction::EaseOut,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(5.0), // 4秒後
        1.0,                             // 不透明度 100%
        EasingFunction::EaseIn,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(6.0), // 5秒後
        0.0,                             // 不透明度 0%
        EasingFunction::Linear,
    )?;

    // スケールのキーフレームアニメーション（サイズの変化）
    // 開始時: 0.5 (50%サイズ) -> 5秒後: 1.0 (100%サイズ)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "scale",
        TimePosition::from_seconds(1.0), // 開始時点
        0.5,                             // 開始時のスケール (50%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "scale",
        TimePosition::from_seconds(6.0), // 5秒後
        1.0,                             // スケール 100%
        EasingFunction::EaseOut,
    )?;

    // 位置のキーフレームアニメーション（X座標の変化）
    // 開始時: -0.3 (左寄り) -> 5秒後: 0.3 (右寄り)
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "position_x",
        TimePosition::from_seconds(1.0), // 開始時点
        -0.3,                            // 開始時のX位置 (左寄り)
        EasingFunction::EaseInOut,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "position_x",
        TimePosition::from_seconds(6.0), // 5秒後
        0.3,                             // X位置 (右寄り)
        EasingFunction::EaseInOut,
    )?;

    // 出力ファイルのパスを設定
    let output_path = PathBuf::from("output/keyframe_animation_output.mp4");

    // レンダリング設定を作成
    let render_config = RenderConfig::new(output_path.clone())
        .with_resolution(1280, 720)
        .with_frame_rate(30.0)
        .with_video_settings(VideoCodec::H264, 85)
        .with_audio_settings(AudioCodec::AAC, 80)
        .with_format(OutputFormat::MP4);

    println!("Rendering project to: {}", output_path.display());

    // プロジェクトをレンダリング
    let result = project.render_with_config(render_config)?;

    println!("Rendering completed successfully!");
    println!("Duration: {} seconds", result.duration.as_seconds());
    println!("Output file: {}", result.output_path.display());

    Ok(())
}
