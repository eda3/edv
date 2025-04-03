use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
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

        println!("🖼️ Loading frame {}: {:?}", frame_number + 1, frame_path);

        // SDL2でテクスチャを作成
        let img = image::open(&frame_path)
            .map_err(|e| {
                crate::cli::Error::CommandExecution(format!("Failed to load image: {}", e))
            })?
            .to_rgb8();

        println!("🎨 Image loaded: {}x{}", img.width(), img.height());

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

        println!("✅ Texture created successfully");
        Ok(texture)
    }

    /// Plays the video file
    pub fn play(&self, input_file: &Path) -> Result<()> {
        // Extract frames
        println!("🎬 Starting EDV Frame Player with SDL2");
        self.extract_frames(input_file)?;

        // SDL2の初期化
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

        // イベントポンプ
        let mut event_pump = sdl_context.event_pump().map_err(|e| {
            crate::cli::Error::CommandExecution(format!("Event pump creation failed: {}", e))
        })?;

        // 再生状態
        *self.playing.lock().unwrap() = true;
        let playing = Arc::clone(&self.playing);
        let current_frame = Arc::clone(&self.current_frame);
        let total_frames = Arc::clone(&self.total_frames);
        let frame_rate = self.frame_rate;

        // 最初のフレームをロード
        let frame_number = *current_frame.lock().unwrap();
        let mut current_texture = self.load_frame_texture(frame_number, &texture_creator)?;

        // フレーム再生スレッド
        let player_thread = thread::spawn(move || {
            let frame_duration = Duration::from_secs_f64(1.0 / frame_rate);

            while *playing.lock().unwrap() {
                let start = Instant::now();

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

                // フレームレートの維持
                let elapsed = start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
        });

        // メインループ
        let mut last_frame = frame_number;
        'running: loop {
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
                            let mut playing_lock = self.playing.lock().unwrap();
                            *playing_lock = !*playing_lock;
                            println!(
                                "▶️ Playback {}",
                                if *playing_lock { "resumed" } else { "paused" }
                            );
                        }
                        Keycode::Period => {
                            let mut frame = self.current_frame.lock().unwrap();
                            let total = *self.total_frames.lock().unwrap();
                            if *frame < total - 1 {
                                *frame += 1;
                            }
                        }
                        Keycode::Comma => {
                            let mut frame = self.current_frame.lock().unwrap();
                            if *frame > 0 {
                                *frame -= 1;
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
                match self.load_frame_texture(current, &texture_creator) {
                    Ok(texture) => {
                        current_texture = texture;
                        // ウィンドウタイトルの更新
                        let total = *self.total_frames.lock().unwrap();
                        let window = canvas.window_mut();
                        window
                            .set_title(&format!(
                                "{} - Frame {}/{} - {} FPS",
                                self.title,
                                current + 1,
                                total,
                                self.frame_rate
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

            // 少し休む
            thread::sleep(Duration::from_millis(10));
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
