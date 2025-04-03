use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::Result;
use crate::ffmpeg::FFmpeg;

/// A simple frame-based video player that uses FFmpeg to extract frames
/// and display them, creating a basic GUI experience without external
/// GUI libraries.
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

    /// Adds overlays to the extracted frames
    fn add_overlay_to_frames(&self) -> Result<()> {
        println!("🖌️ Adding overlays to frames...");

        let frames_path = self.temp_dir.join("frame_%04d.jpg");
        let overlay_path = self.temp_dir.join("overlay_%04d.jpg");

        let mut cmd = Command::new(self.ffmpeg.path());
        cmd.arg("-i")
            .arg(frames_path.to_str().unwrap())
            .arg("-vf")
            // Add information overlay
            .arg(concat!(
                "drawtext=text='%{n} / %{frame_num}':x=10:y=10:fontcolor=white:fontsize=24,",
                "drawtext=text='EDV Player':x=w-tw-10:y=10:fontcolor=white:fontsize=24,",
                "drawbox=x=10:y=h-30:w=w-20:h=20:color=blue@0.5:t=2"
            ))
            .arg("-q:v")
            .arg("3")
            .arg(overlay_path.to_str().unwrap());

        let output = cmd.output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(crate::cli::Error::CommandExecution(format!(
                "Adding overlays failed: {}",
                error
            )));
        }

        println!("✅ Overlays added successfully");

        Ok(())
    }

    /// Updates the console UI
    fn update_console_ui(&self) -> Result<()> {
        // Clear terminal
        print!("\x1B[2J\x1B[1;1H");

        // Display UI
        println!("┌──────────────────────────────────────────────┐");
        println!("│  {} ", self.title);
        println!("├──────────────────────────────────────────────┤");

        // Progress bar showing current position
        let current = *self.current_frame.lock().unwrap();
        let total = *self.total_frames.lock().unwrap();
        let progress = if total > 0 { current * 30 / total } else { 0 };

        print!("│  [");
        for i in 0..30 {
            if i < progress {
                print!("=");
            } else if i == progress {
                print!(">");
            } else {
                print!(" ");
            }
        }
        println!("] {}/{}", current + 1, total);

        // Controls
        println!("├──────────────────────────────────────────────┤");
        println!("│  Space: Play/Pause  ←/→: Previous/Next frame │");
        println!("│  q: Quit  f: Toggle fullscreen               │");
        println!("└──────────────────────────────────────────────┘");

        Ok(())
    }

    /// Displays the current frame in an image viewer
    fn display_current_frame(&self) -> Result<()> {
        let current = *self.current_frame.lock().unwrap();
        let frame_path = self.temp_dir.join(format!("frame_{:04}.jpg", current + 1));

        if !frame_path.exists() {
            return Err(crate::cli::Error::InvalidPath(format!(
                "Frame not found: {:?}",
                frame_path
            )));
        }

        // Launch viewer based on OS
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "start", "", frame_path.to_str().unwrap()])
                .spawn()?;
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(frame_path).spawn()?;
        } else {
            // Try common Linux image viewers
            let viewers = ["xdg-open", "display", "eog", "feh"];
            let mut success = false;

            for viewer in viewers {
                if let Ok(status) = Command::new(viewer)
                    .arg(&frame_path)
                    .spawn()
                    .and_then(|mut child| child.wait())
                {
                    if status.success() {
                        success = true;
                        break;
                    }
                }
            }

            if !success {
                return Err(crate::cli::Error::CommandExecution(
                    "Failed to open image viewer".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Plays the video file
    pub fn play(&self, input_file: &Path) -> Result<()> {
        // Extract frames
        println!("🎬 Starting EDV Frame Player");
        self.extract_frames(input_file)?;

        // Add overlays to frames (optional feature)
        // self.add_overlay_to_frames()?;

        // Start playback
        *self.playing.lock().unwrap() = true;

        // Run player loop in separate thread
        let playing = Arc::clone(&self.playing);
        let current_frame = Arc::clone(&self.current_frame);
        let total_frames = Arc::clone(&self.total_frames);
        let frame_rate = self.frame_rate;

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
                        // Loop playback
                        *frame = 0;
                    }
                }

                // Maintain frame rate
                let elapsed = start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
        });

        // First display of the current frame
        self.display_current_frame()?;

        // Main thread handles UI updates and input processing
        self.handle_input()?;

        // Cleanup
        *self.playing.lock().unwrap() = false;
        player_thread.join().unwrap();

        println!("👋 EDV Frame Player closed");

        Ok(())
    }

    /// Handles keyboard input
    fn handle_input(&self) -> Result<()> {
        // Note: This is a simplified input handler
        // In a real implementation, you might want to use a crate like
        // crossterm for better terminal input handling

        println!("💡 Control with keyboard: Space to play/pause, q to quit");

        let stdin = std::io::stdin();

        loop {
            // Update UI
            self.update_console_ui()?;

            // Check for input (non-blocking)
            let mut input = String::new();
            if let Ok(_) = stdin.read_line(&mut input) {
                match input.trim() {
                    "q" | "Q" => break,
                    " " => {
                        let mut playing = self.playing.lock().unwrap();
                        *playing = !*playing;
                        println!(
                            "▶️ Playback {}",
                            if *playing { "resumed" } else { "paused" }
                        );
                    }
                    ">" | "." => {
                        let mut frame = self.current_frame.lock().unwrap();
                        let total = *self.total_frames.lock().unwrap();
                        if *frame < total - 1 {
                            *frame += 1;
                            self.display_current_frame()?;
                        }
                    }
                    "<" | "," => {
                        let mut frame = self.current_frame.lock().unwrap();
                        if *frame > 0 {
                            *frame -= 1;
                            self.display_current_frame()?;
                        }
                    }
                    _ => {}
                }
            }

            // Small delay to prevent CPU hogging
            thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }
}

impl Drop for FramePlayer {
    fn drop(&mut self) {
        // Attempt to clean up temporary files when the player is dropped
        if self.temp_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&self.temp_dir) {
                eprintln!("Warning: Failed to clean up temporary directory: {}", e);
            }
        }
    }
}
