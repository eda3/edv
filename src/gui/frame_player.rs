use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::Result;
use crate::ffmpeg::FFmpeg;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

/// A simple frame-based video player that uses FFmpeg to extract frames
/// and display them using SDL2, creating a lightweight GUI experience.
pub struct FramePlayer {
    ffmpeg: FFmpeg,
    frame_width: u32,
    frame_height: u32,
    title: String,
    frame_rate: f64,
    temp_dir: PathBuf,
    playing: Arc<Mutex<bool>>,
    paused: Arc<(Mutex<bool>, Condvar)>, // 一時停止状態とCondVar
    current_frame: Arc<Mutex<usize>>,
    total_frames: Arc<Mutex<usize>>,
    preloaded_count: Arc<Mutex<usize>>, // プリロード数を追跡
}

impl FramePlayer {
    /// Creates a new FramePlayer instance
    pub fn new(ffmpeg: FFmpeg) -> Self {
        let temp_dir = std::env::temp_dir().join("edv_player");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temporary directory");

        Self {
            ffmpeg,
            frame_width: 800,
            frame_height: 600,
            title: "EDV Player".to_string(),
            frame_rate: 30.0,
            temp_dir,
            playing: Arc::new(Mutex::new(false)),
            paused: Arc::new((Mutex::new(false), Condvar::new())), // 一時停止状態の初期化
            current_frame: Arc::new(Mutex::new(0)),
            total_frames: Arc::new(Mutex::new(0)),
            preloaded_count: Arc::new(Mutex::new(10)), // デフォルトで10フレーム先読み
        }
    }

    /// Sets the player title
    pub fn set_title(&mut self, title: &str) -> &mut Self {
        self.title = title.to_string();
        self
    }

    /// Sets the frame size
    pub fn set_frame_size(&mut self, width: u32, height: u32) -> &mut Self {
        self.frame_width = width;
        self.frame_height = height;
        self
    }

    /// Sets the frame rate
    pub fn set_frame_rate(&mut self, fps: f64) -> &mut Self {
        self.frame_rate = fps;
        self
    }

    /// Extracts frames from the input video file
    fn extract_frames(&self, input_file: &Path) -> Result<()> {
        // Clean temporary folder
        if self.temp_dir.exists() {
            for entry in std::fs::read_dir(&self.temp_dir)? {
                if let Ok(entry) = entry {
                    if let Err(e) = std::fs::remove_file(entry.path()) {
                        eprintln!("Warning: Failed to remove file: {}", e);
                    }
                }
            }
        } else {
            std::fs::create_dir_all(&self.temp_dir)?;
        }

        println!("🎬 Extracting frames to: {:?}", self.temp_dir);

        // Frame extraction command
        let mut cmd = Command::new(self.ffmpeg.path());
        cmd.arg("-i")
            .arg(input_file)
            .arg("-vf")
            .arg(format!("fps={}", self.frame_rate))
            .arg("-s")
            .arg(format!("{}x{}", self.frame_width, self.frame_height))
            .arg("-q:v")
            .arg("3") // High quality
            .arg(self.temp_dir.join("frame_%04d.jpg").to_str().unwrap());

        // Execute extraction
        let output = cmd.output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(crate::cli::Error::CommandExecution(format!(
                "Frame extraction failed: {}",
                error
            )));
        }

        // Count total frames
        let frames = std::fs::read_dir(&self.temp_dir)?
            .filter_map(|r| r.ok())
            .filter(|entry| {
                entry
                    .path()
                    .file_name()
                    .map(|name| name.to_string_lossy().starts_with("frame_"))
                    .unwrap_or(false)
            })
            .count();

        println!("✅ Extracted {} frames", frames);

        let mut total = self.total_frames.lock().unwrap();
        *total = frames;

        Ok(())
    }

    /// Loads a frame as an SDL2 texture
    fn load_frame_texture<'a>(
        &self,
        frame_number: usize,
        texture_creator: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>,
        preloaded_frames: &mut HashSet<usize>, // どのフレームを既に読み込んだかを記録
    ) -> Result<sdl2::render::Texture<'a>> {
        let frame_path = self
            .temp_dir
            .join(format!("frame_{:04}.jpg", frame_number + 1));

        if !frame_path.exists() {
            return Err(crate::cli::Error::InvalidPath(format!(
                "Frame not found: {:?}",
                frame_path
            )));
        }

        // 初めて読み込むフレームの場合だけログ出力
        if !preloaded_frames.contains(&frame_number) {
            if frame_number % 30 == 0 {
                println!("🖼️ Loading frame {}", frame_number + 1);
            }
            preloaded_frames.insert(frame_number);
        }

        // SDL2でテクスチャを作成
        let img = image::open(&frame_path)
            .map_err(|e| {
                crate::cli::Error::CommandExecution(format!("Failed to load image: {}", e))
            })?
            .to_rgb8();

        // SDL2テクスチャの作成
        let mut texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, img.width(), img.height())
            .map_err(|e| {
                crate::cli::Error::CommandExecution(format!("Failed to create texture: {}", e))
            })?;

        // テクスチャにピクセルデータをコピー
        texture
            .update(
                None,
                &img.as_raw(),
                img.width() as usize * 3, // RGB24なので1ピクセルあたり3バイト
            )
            .map_err(|e| {
                crate::cli::Error::CommandExecution(format!("Failed to update texture: {}", e))
            })?;

        Ok(texture)
    }

    /// Plays the video file
    pub fn play(&self, input_file: &Path) -> Result<()> {
        // Extract frames
        println!("🎬 Starting EDV Frame Player with SDL2");
        self.extract_frames(input_file)?;

        // SDL2の初期化と設定
        let sdl_context = sdl2::init()
            .map_err(|e| crate::cli::Error::CommandExecution(format!("SDL2 init failed: {}", e)))?;

        let video_subsystem = sdl_context.video().map_err(|e| {
            crate::cli::Error::CommandExecution(format!("Video init failed: {}", e))
        })?;

        // ウィンドウ作成
        let window = video_subsystem
            .window(&self.title, self.frame_width, self.frame_height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| {
                crate::cli::Error::CommandExecution(format!("Window creation failed: {}", e))
            })?;

        // レンダラー作成
        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build() // VSyncを無効化して応答性を向上
            .map_err(|e| {
                crate::cli::Error::CommandExecution(format!("Renderer creation failed: {}", e))
            })?;

        // テクスチャクリエイター
        let texture_creator = canvas.texture_creator();

        // 既にロードしたフレームを記録
        let mut preloaded_frames: HashSet<usize> = HashSet::new();

        // イベントポンプ
        let mut event_pump = sdl_context.event_pump().map_err(|e| {
            crate::cli::Error::CommandExecution(format!("Event pump creation failed: {}", e))
        })?;

        // 再生状態
        *self.playing.lock().unwrap() = true;
        *self.paused.0.lock().unwrap() = false; // 初期状態では一時停止していない
        let playing = Arc::clone(&self.playing);
        let paused = Arc::clone(&self.paused);
        let current_frame = Arc::clone(&self.current_frame);
        let total_frames = Arc::clone(&self.total_frames);
        let preloaded_count = Arc::clone(&self.preloaded_count);
        let frame_rate = self.frame_rate;

        // テクスチャキャッシュ用のハッシュマップ（メモリ効率を考慮して最大10フレーム保持）
        let texture_cache_size = 10;
        let mut texture_cache: HashMap<usize, sdl2::render::Texture> =
            HashMap::with_capacity(texture_cache_size);
        let temp_dir = self.temp_dir.clone();

        // プリロードスレッド（先行してフレームをロード）
        let preload_thread = {
            let current_frame = Arc::clone(&current_frame);
            let total_frames = Arc::clone(&total_frames);
            let preloaded_count = Arc::clone(&preloaded_count);
            let playing = Arc::clone(&playing);
            let paused = Arc::clone(&paused);
            let temp_dir = temp_dir.clone();

            thread::spawn(move || {
                let mut loaded_frames: HashSet<usize> = HashSet::new();

                while *playing.lock().unwrap() {
                    // 一時停止中も先読みは行う
                    let is_paused = paused.0.lock().unwrap().clone();
                    let frame = *current_frame.lock().unwrap();
                    let total = *total_frames.lock().unwrap();
                    let ahead_count = *preloaded_count.lock().unwrap();

                    // 現在位置から数フレーム先までを先読み
                    for offset in 1..=ahead_count {
                        let target_frame = frame + offset;
                        if target_frame >= total {
                            break;
                        }

                        // まだロードしていないフレームならファイルを開いておく
                        if !loaded_frames.contains(&target_frame) {
                            let frame_path =
                                temp_dir.join(format!("frame_{:04}.jpg", target_frame + 1));

                            if frame_path.exists() {
                                // 実際にファイルを開いてキャッシュにのせる
                                match image::open(&frame_path) {
                                    Ok(_) => {
                                        loaded_frames.insert(target_frame);
                                        if loaded_frames.len() > texture_cache_size * 3 {
                                            // メモリ使用量を抑えるため、古いエントリを削除
                                            if let Some(oldest) =
                                                loaded_frames.iter().min().cloned()
                                            {
                                                if oldest < frame {
                                                    loaded_frames.remove(&oldest);
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }

                    // 一時停止中も定期的にチェック
                    thread::sleep(Duration::from_millis(20));
                }
            })
        };

        // 最初のフレームをロード
        let frame_number = *current_frame.lock().unwrap();
        let mut current_texture =
            self.load_frame_texture(frame_number, &texture_creator, &mut preloaded_frames)?;

        // フレーム更新スレッドの最適化
        let player_thread = thread::spawn(move || {
            let frame_duration = Duration::from_secs_f64(1.0 / frame_rate);
            let mut last_frame_time = Instant::now();

            // 外側のループは再生が完全に終了するまで継続
            while *playing.lock().unwrap() {
                // ポーズ状態をチェック
                let &(ref paused_lock, ref cvar) = &*paused;
                let mut is_paused = paused_lock.lock().unwrap();

                // 一時停止中はCondVarで待機（CPUを使わない）
                if *is_paused {
                    // 通知があるまで待機
                    is_paused = cvar.wait(is_paused).unwrap();

                    // 再開時にタイマーをリセット（重要！）
                    last_frame_time = Instant::now();

                    // 再開時にplayingをチェック（終了フラグ）
                    if !*playing.lock().unwrap() {
                        return;
                    }

                    // 少し待ってから継続（安定性向上）
                    thread::sleep(Duration::from_millis(5));

                    // 次のイテレーションに行く
                    continue;
                }

                // 現在の時間を取得
                let now = Instant::now();
                let elapsed = now.duration_since(last_frame_time);

                // フレームレートに従ってフレームを進める
                if elapsed >= frame_duration {
                    // 次のフレームに進む
                    {
                        let mut frame = current_frame.lock().unwrap();
                        let total = *total_frames.lock().unwrap();

                        if *frame < total - 1 {
                            *frame += 1;
                        } else {
                            // ループ再生
                            *frame = 0;
                        }
                    }

                    // タイマーを更新
                    last_frame_time = now;
                } else {
                    // まだ次のフレームの時間になっていない
                    let wait_time = frame_duration
                        .checked_sub(elapsed)
                        .unwrap_or(Duration::from_millis(1));
                    thread::sleep(wait_time);
                }
            }
        });

        // メインループ
        let mut last_frame = frame_number;
        let mut fps_timer = Instant::now();
        let mut frames_counted = 0;
        let mut current_fps = 0.0;

        // タイムラインドラッグ用の状態管理
        let mut is_dragging = false;
        let mut drag_target_frame = 0;
        let mut last_drag_update = Instant::now();
        let drag_update_interval = Duration::from_millis(50); // ドラッグ中の更新間隔

        'running: loop {
            // FPS計測（デバッグ用）
            frames_counted += 1;
            let elapsed = fps_timer.elapsed();
            if elapsed >= Duration::from_secs(1) {
                current_fps = frames_counted as f64 / elapsed.as_secs_f64();
                frames_counted = 0;
                fps_timer = Instant::now();
            }

            // イベント処理
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Q => break 'running,
                        Keycode::Space => {
                            // 完全に新しい一時停止/再開処理
                            let is_currently_paused = {
                                // スコープを限定して短時間だけロック保持
                                self.paused.0.lock().unwrap().clone()
                            };

                            // 状態を反転
                            {
                                let mut paused_guard = match self.paused.0.try_lock() {
                                    Ok(guard) => guard,
                                    Err(_) => {
                                        // もう一度試す - より強力なlock()を使用
                                        println!("🔄 ロック競合を検出、再試行...");
                                        thread::sleep(Duration::from_millis(5));
                                        self.paused.0.lock().unwrap()
                                    }
                                };

                                // 値を反転
                                *paused_guard = !is_currently_paused;

                                // ユーザーにフィードバック
                                if *paused_guard {
                                    println!("⏸️ 一時停止");
                                } else {
                                    println!("▶️ 再生再開");
                                }
                            }

                            // スレッドに通知（ロック範囲外で実行）
                            self.paused.1.notify_all();

                            // 画面を即時更新して応答性を高める
                            canvas.present();
                        }
                        Keycode::Period => {
                            let mut frame = self.current_frame.lock().unwrap();
                            let total = *self.total_frames.lock().unwrap();
                            if *frame < total - 1 {
                                *frame += 1;
                                println!("⏭️ 次のフレーム: {}", *frame + 1);
                            }
                        }
                        Keycode::Comma => {
                            let mut frame = self.current_frame.lock().unwrap();
                            if *frame > 0 {
                                *frame -= 1;
                                println!("⏮️ 前のフレーム: {}", *frame + 1);
                            }
                        }
                        Keycode::Right => {
                            let mut frame = self.current_frame.lock().unwrap();
                            let total = *self.total_frames.lock().unwrap();
                            if *frame < total - 1 {
                                *frame += 1;
                                println!("⏭️ 次のフレーム: {}", *frame + 1);

                                // 一時停止中でも即座にフレームを更新
                                if let Ok(texture) = self.load_frame_texture(
                                    *frame,
                                    &texture_creator,
                                    &mut preloaded_frames,
                                ) {
                                    current_texture = texture;
                                }
                            }
                        }
                        Keycode::Left => {
                            let mut frame = self.current_frame.lock().unwrap();
                            if *frame > 0 {
                                *frame -= 1;
                                println!("⏮️ 前のフレーム: {}", *frame + 1);

                                // 一時停止中でも即座にフレームを更新
                                if let Ok(texture) = self.load_frame_texture(
                                    *frame,
                                    &texture_creator,
                                    &mut preloaded_frames,
                                ) {
                                    current_texture = texture;
                                }
                            }
                        }
                        _ => {}
                    },
                    Event::MouseButtonDown {
                        x, y, mouse_btn, ..
                    } => {
                        // タイムラインエリアをクリックした場合
                        let frame_height = self.frame_height - 30;
                        if y > frame_height as i32
                            && y < self.frame_height as i32
                            && mouse_btn == sdl2::mouse::MouseButton::Left
                        {
                            is_dragging = true; // ドラッグ開始

                            let total = *self.total_frames.lock().unwrap();
                            if total > 0 {
                                // クリック位置からフレーム位置を計算
                                let click_ratio = x as f32 / self.frame_width as f32;
                                let target_frame = (click_ratio * total as f32) as usize;
                                let target_frame = target_frame.min(total - 1);

                                // フレーム位置を更新
                                let mut frame = self.current_frame.lock().unwrap();
                                *frame = target_frame;
                                println!("🔍 ジャンプ: フレーム {}", target_frame + 1);

                                // 即座にフレームを更新
                                if let Ok(texture) = self.load_frame_texture(
                                    target_frame,
                                    &texture_creator,
                                    &mut preloaded_frames,
                                ) {
                                    current_texture = texture;
                                }
                            }
                        }
                    }
                    Event::MouseButtonUp { mouse_btn, .. } => {
                        // ドラッグ終了
                        if mouse_btn == sdl2::mouse::MouseButton::Left && is_dragging {
                            is_dragging = false;

                            // ドラッグ完了時に最終位置のフレームを読み込み
                            if drag_target_frame > 0 {
                                let mut frame = self.current_frame.lock().unwrap();
                                *frame = drag_target_frame;

                                // 本格的なフレーム読み込み（ドラッグ完了時のみ）
                                if let Ok(texture) = self.load_frame_texture(
                                    drag_target_frame,
                                    &texture_creator,
                                    &mut preloaded_frames,
                                ) {
                                    current_texture = texture;
                                    println!("🔍 ドラッグ完了: フレーム {}", drag_target_frame + 1);
                                }
                            }
                        }
                    }
                    Event::MouseMotion { x, y, .. } => {
                        // ドラッグ中の処理
                        if is_dragging {
                            // タイムラインエリア内かチェック
                            let frame_height = self.frame_height - 30;
                            if y > frame_height as i32 && y < self.frame_height as i32 {
                                let total = *self.total_frames.lock().unwrap();
                                if total > 0 {
                                    // ドラッグ位置からフレーム位置を計算
                                    let drag_ratio =
                                        (x.max(0) as f32 / self.frame_width as f32).min(1.0);
                                    let target_frame = (drag_ratio * total as f32) as usize;
                                    let target_frame = target_frame.min(total - 1);

                                    // ターゲットフレームを保存（ドラッグ終了時に適用）
                                    drag_target_frame = target_frame;

                                    // 前回の更新から一定時間経過した場合のみ更新（パフォーマンス向上）
                                    let now = Instant::now();
                                    if now.duration_since(last_drag_update) >= drag_update_interval
                                    {
                                        last_drag_update = now;

                                        // フレーム位置を更新
                                        let mut frame = self.current_frame.lock().unwrap();
                                        if *frame != target_frame {
                                            *frame = target_frame;

                                            // 再生が一時停止されていることを確認（一度だけ）
                                            if let Ok(mut is_paused) = self.paused.0.try_lock() {
                                                if !*is_paused {
                                                    *is_paused = true;
                                                    println!("⏸️ ドラッグ時に一時停止");
                                                    // 再生スレッドに通知
                                                    self.paused.1.notify_all();
                                                }
                                            }
                                        }
                                    }

                                    // タイムラインの視覚的な位置だけを更新（軽量）
                                    let progress = drag_ratio;

                                    // 描画
                                    canvas.clear();

                                    // ビデオフレームはそのまま
                                    let frame_height = self.frame_height - 30;
                                    let dst = Rect::new(0, 0, self.frame_width, frame_height);
                                    canvas
                                        .copy(&current_texture, None, Some(dst))
                                        .unwrap_or_default();

                                    // タイムラインの背景（グレー）
                                    canvas.set_draw_color(sdl2::pixels::Color::RGB(60, 60, 60));
                                    let timeline_bg =
                                        Rect::new(0, frame_height as i32 + 5, self.frame_width, 20);
                                    canvas.fill_rect(timeline_bg).unwrap_or_default();

                                    // 進行状況インジケータ（水色）
                                    canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 170, 255));
                                    let progress_width =
                                        (self.frame_width as f32 * progress) as u32;
                                    let timeline_progress =
                                        Rect::new(0, frame_height as i32 + 5, progress_width, 20);
                                    canvas.fill_rect(timeline_progress).unwrap_or_default();

                                    // 現在位置マーカー（白）
                                    canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                                    let marker_x = (self.frame_width as f32 * progress) as i32;
                                    let marker =
                                        Rect::new(marker_x - 2, frame_height as i32 + 3, 4, 24);
                                    canvas.fill_rect(marker).unwrap_or_default();

                                    // フレーム番号表示（ドラッグ中のフレーム番号）
                                    canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                                    let window = canvas.window_mut();
                                    window
                                        .set_title(&format!(
                                            "{} - Frame {}/{} (ドラッグ中)",
                                            self.title,
                                            target_frame + 1,
                                            total
                                        ))
                                        .unwrap_or_default();

                                    canvas.present();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // フレーム更新
            let current = *self.current_frame.lock().unwrap();
            if current != last_frame {
                // 新しいフレームをロード
                match self.load_frame_texture(current, &texture_creator, &mut preloaded_frames) {
                    Ok(texture) => {
                        current_texture = texture;
                        // ウィンドウタイトルの更新（FPSを含む）
                        let total = *self.total_frames.lock().unwrap();
                        let window = canvas.window_mut();
                        window
                            .set_title(&format!(
                                "{} - Frame {}/{} - {:.1} FPS",
                                self.title,
                                current + 1,
                                total,
                                current_fps
                            ))
                            .unwrap_or_default();
                    }
                    Err(e) => println!("Error loading frame: {:?}", e),
                }
                last_frame = current;
            }

            // 描画
            canvas.clear();

            // ビデオフレームを描画
            let frame_height = self.frame_height - 30; // タイムライン用に30ピクセル確保
            let dst = Rect::new(0, 0, self.frame_width, frame_height);
            canvas
                .copy(&current_texture, None, Some(dst))
                .unwrap_or_default();

            // タイムラインの描画
            let total = *self.total_frames.lock().unwrap();
            let progress = current as f32 / (total as f32).max(1.0);

            // タイムラインの背景（グレー）
            canvas.set_draw_color(sdl2::pixels::Color::RGB(60, 60, 60));
            let timeline_bg = Rect::new(0, frame_height as i32 + 5, self.frame_width, 20);
            canvas.fill_rect(timeline_bg).unwrap_or_default();

            // 進行状況インジケータ（水色）
            canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 170, 255));
            let progress_width = (self.frame_width as f32 * progress) as u32;
            let timeline_progress = Rect::new(0, frame_height as i32 + 5, progress_width, 20);
            canvas.fill_rect(timeline_progress).unwrap_or_default();

            // 現在位置マーカー（白）
            canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
            let marker_x = (self.frame_width as f32 * progress) as i32;
            let marker = Rect::new(marker_x - 2, frame_height as i32 + 3, 4, 24);
            canvas.fill_rect(marker).unwrap_or_default();

            // フレーム番号を表示（テキスト）
            let frame_info = format!("{}/{}", current + 1, total);

            canvas.present();

            // フレームレートを制御（メインループも適切な速度で実行）
            thread::sleep(Duration::from_millis(5));
        }

        // クリーンアップ
        *self.playing.lock().unwrap() = false;
        player_thread.join().unwrap();

        println!("👋 EDV Frame Player closed");
        Ok(())
    }
}

impl Drop for FramePlayer {
    fn drop(&mut self) {
        // プレーヤーが削除されたときに一時ファイルをクリーンアップ
        if self.temp_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&self.temp_dir) {
                eprintln!("Warning: Failed to clean up temporary directory: {}", e);
            }
        }
    }
}
