/// 高度なマルチトラック編集のサンプルプロジェクト
///
/// このサンプルでは、複数のビデオトラックとオーディオトラックを組み合わせ、
/// トラックの可視性やロック機能を使用した高度な編集プロジェクトを作成します。
use std::path::{Path, PathBuf};

use edv::ffmpeg::FFmpeg;
use edv::project::rendering::{AudioCodec, OutputFormat, RenderConfig, VideoCodec};
use edv::project::timeline::keyframes::EasingFunction;
use edv::project::timeline::multi_track::TrackRelationship;
use edv::project::timeline::{Clip, TrackKind};
use edv::project::{AssetId, AssetMetadata, AssetReference, ClipId, Project};
use edv::utility::time::{Duration, TimePosition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // FFmpegを初期化
    let ffmpeg = FFmpeg::detect()?;
    println!("FFmpeg detected: {}", ffmpeg.path().display());
    println!("FFmpeg version: {}", ffmpeg.version());

    // プロジェクトを作成
    let mut project = Project::new("高度なマルチトラックサンプル");

    // 入力ファイルのパスを設定
    let input_file = PathBuf::from("test_media/sozai.mp4");
    if !input_file.exists() {
        return Err(format!("Input file not found: {}", input_file.display()).into());
    }

    // 入力ファイルのメディア情報を取得
    let media_info = ffmpeg.get_media_info(&input_file)?;

    // メディア情報から基本的なデータを取得
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
    let main_asset_id = project.add_asset(input_file.clone(), asset_metadata.clone());

    // 2つ目のアセットとして同じファイルを追加 (オーバーレイ用)
    asset_metadata
        .extra
        .insert("purpose".to_string(), "overlay".to_string());
    let overlay_asset_id = project.add_asset(input_file.clone(), asset_metadata.clone());

    // 3つ目のアセットとして同じファイルを追加 (バックグラウンド用)
    asset_metadata
        .extra
        .insert("purpose".to_string(), "background".to_string());
    let background_asset_id = project.add_asset(input_file.clone(), asset_metadata);

    // タイムラインにトラックを追加
    let background_track_id = project.timeline.add_track(TrackKind::Video); // 背景ビデオ
    let main_video_track_id = project.timeline.add_track(TrackKind::Video); // メインビデオ
    let overlay_video_track_id = project.timeline.add_track(TrackKind::Video); // オーバーレイビデオ
    let main_audio_track_id = project.timeline.add_track(TrackKind::Audio); // メインオーディオ
    let secondary_audio_track_id = project.timeline.add_track(TrackKind::Audio); // 効果音用オーディオ

    // トラック名を設定
    if let Some(track) = project.timeline.get_track_mut(background_track_id) {
        track.set_name("背景ビデオ");
    }

    if let Some(track) = project.timeline.get_track_mut(main_video_track_id) {
        track.set_name("メインビデオ");
    }

    if let Some(track) = project.timeline.get_track_mut(overlay_video_track_id) {
        track.set_name("オーバーレイビデオ");
    }

    if let Some(track) = project.timeline.get_track_mut(main_audio_track_id) {
        track.set_name("メインオーディオ");
    }

    if let Some(track) = project.timeline.get_track_mut(secondary_audio_track_id) {
        track.set_name("効果音");
    }

    // 背景ビデオクリップを作成（全体をカバー）
    let background_clip = Clip::new(
        ClipId::new(),
        background_asset_id,
        TimePosition::from_seconds(0.0),
        Duration::from_seconds(main_video_duration + 2.0), // 少し長めに
        TimePosition::from_seconds(0.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project
        .timeline
        .add_clip(background_track_id, background_clip)?;

    // メインビデオクリップ（2秒遅れで開始）
    let main_clip = Clip::new(
        ClipId::new(),
        main_asset_id,
        TimePosition::from_seconds(2.0), // 2秒後から開始
        Duration::from_seconds(main_video_duration - 2.0), // 少し短く
        TimePosition::from_seconds(1.0), // 元動画の1秒目から
        TimePosition::from_seconds(main_video_duration),
    );

    project.timeline.add_clip(main_video_track_id, main_clip)?;

    // オーバーレイビデオクリップ（4秒時点で3秒間表示）
    let overlay_clip = Clip::new(
        ClipId::new(),
        overlay_asset_id,
        TimePosition::from_seconds(4.0), // 4秒後から開始
        Duration::from_seconds(3.0),     // 3秒間表示
        TimePosition::from_seconds(2.0), // 元動画の2秒目から
        TimePosition::from_seconds(5.0), // 元動画の5秒目まで
    );

    project
        .timeline
        .add_clip(overlay_video_track_id, overlay_clip)?;

    // メインオーディオクリップ（メインビデオと同じタイミング）
    let main_audio_clip = Clip::new(
        ClipId::new(),
        main_asset_id, // 同じアセットのオーディオを使用
        TimePosition::from_seconds(2.0),
        Duration::from_seconds(main_video_duration - 2.0),
        TimePosition::from_seconds(1.0),
        TimePosition::from_seconds(main_video_duration),
    );

    project
        .timeline
        .add_clip(main_audio_track_id, main_audio_clip)?;

    // 効果音用オーディオクリップ（元の動画の一部を繰り返し）
    let effect_audio_clip = Clip::new(
        ClipId::new(),
        main_asset_id,                   // 同じアセットのオーディオを使用
        TimePosition::from_seconds(4.0), // 4秒後から
        Duration::from_seconds(3.0),     // 3秒間
        TimePosition::from_seconds(0.0), // 元動画の最初から
        TimePosition::from_seconds(3.0), // 元動画の3秒目まで
    );

    project
        .timeline
        .add_clip(secondary_audio_track_id, effect_audio_clip)?;

    // トラックのミュート状態とロック状態を設定
    if let Some(track) = project.timeline.get_track_mut(secondary_audio_track_id) {
        track.set_muted(true); // 効果音トラックをミュート（レンダリング時に含まない）
    }

    if let Some(track) = project.timeline.get_track_mut(background_track_id) {
        track.set_locked(true); // 背景トラックをロック
    }

    // 背景トラックの不透明度をキーフレームで設定（30%）
    project.timeline.add_keyframe_with_history(
        background_track_id,
        "opacity",
        TimePosition::from_seconds(0.0),
        0.3, // 30%の不透明度
        EasingFunction::Linear,
    )?;

    // オーバーレイビデオの不透明度をキーフレームで設定（70%）
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(0.0),
        0.7, // 70%の不透明度
        EasingFunction::Linear,
    )?;

    // オーバーレイビデオのキーフレームアニメーションを追加
    // 不透明度のキーフレームアニメーション
    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(4.0), // 開始時点
        0.0,                             // 開始時の不透明度 (0%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(5.0), // 1秒後
        0.7,                             // 不透明度 70%
        EasingFunction::EaseOut,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(6.0), // 2秒後
        0.7,                             // 不透明度 70%
        EasingFunction::EaseIn,
    )?;

    project.timeline.add_keyframe_with_history(
        overlay_video_track_id,
        "opacity",
        TimePosition::from_seconds(7.0), // 3秒後
        0.0,                             // 不透明度 0%
        EasingFunction::Linear,
    )?;

    // メインオーディオのボリュームキーフレーム（フェードイン・フェードアウト）
    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(2.0), // 開始時点
        0.0,                             // 開始時のボリューム (0%)
        EasingFunction::Linear,
    )?;

    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(3.0), // 1秒後
        1.0,                             // ボリューム 100%
        EasingFunction::EaseOut,
    )?;

    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(main_video_duration - 1.0), // 終了1秒前
        1.0,                                                   // ボリューム 100%
        EasingFunction::EaseIn,
    )?;

    project.timeline.add_keyframe_with_history(
        main_audio_track_id,
        "volume",
        TimePosition::from_seconds(main_video_duration), // 終了時点
        0.0,                                             // ボリューム 0%
        EasingFunction::Linear,
    )?;

    // トラック間の関係を設定（メインビデオとメインオーディオをリンク）
    // 注：コンパイルエラーを修正するためコメントアウト
    // このコードはRustの所有権とライフタイムの複雑な問題があるため、サンプルでは省略します
    /*
    {
        // いったんクローンを作成してマルチトラックマネージャーを取得
        let timeline_ref = project.timeline.clone();
        let mut multi_track_manager = timeline_ref.multi_track_manager_mut();

        // トラック間の関係を設定（メインビデオとメインオーディオをリンク）
        multi_track_manager.add_relationship(
            main_video_track_id,
            main_audio_track_id,
            TrackRelationship::Locked,
            &timeline_ref,
        )?;
    }
    */

    // 出力ファイルのパスを設定
    let output_path = PathBuf::from("output/advanced_multi_track_output.mp4");

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
