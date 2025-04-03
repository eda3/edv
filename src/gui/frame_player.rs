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
    paused: Arc<(Mutex<bool>, Condvar)>, // 一時停止状態とCondvar
    current_frame: Arc<Mutex<usize>>,
    total_frames: Arc<Mutex<usize>>,
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

        // SDL2の初期化と設定（既存のコードと同じ）
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
            .present_vsync()
            .build()
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
        let frame_rate = self.frame_rate;

        // 最初のフレームをロード
        let frame_number = *current_frame.lock().unwrap();
        let mut current_texture =
            self.load_frame_texture(frame_number, &texture_creator, &mut preloaded_frames)?;

        // 改善したフレーム再生スレッド
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
                    last_frame_time = Instant::now()
                        .checked_sub(frame_duration / 2)
                        .unwrap_or_else(Instant::now);

                    // 再開時にplayingをチェック（終了フラグ）
                    if !*playing.lock().unwrap() {
                        return;
                    }

                    // 次のイテレーションに行くが、すぐに次のフレームを表示できるようタイマーを調整
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
                            // 一時停止/再開の改善版
                            let &(ref paused_lock, ref cvar) = &*self.paused;
                            let mut is_paused = paused_lock.lock().unwrap();
                            if *is_paused {
                                println!("▶️ 再生再開");
                                // 再生再開時には即座にフレームを更新するためにシグナルを送る
                                *is_paused = false;
                                cvar.notify_all();

                                // メインスレッドでも即座にフレームを更新
                                let mut frame = self.current_frame.lock().unwrap();
                                let total = *self.total_frames.lock().unwrap();
                                if *frame < total - 1 {
                                    *frame += 1;
                                }
                            } else {
                                println!("⏸️ 一時停止");
                                *is_paused = true;
                            }
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
                            }
                        }
                        Keycode::Left => {
                            let mut frame = self.current_frame.lock().unwrap();
                            if *frame > 0 {
                                *frame -= 1;
                                println!("⏮️ 前のフレーム: {}", *frame + 1);
                            }
                        }
                        _ => {}
                    },
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
            let dst = Rect::new(0, 0, self.frame_width, self.frame_height);
            canvas
                .copy(&current_texture, None, Some(dst))
                .unwrap_or_default();
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
