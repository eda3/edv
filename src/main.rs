use anyhow::Result;
use ffmpeg_next as ffmpeg;
use image::{ImageBuffer, Rgb};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// 動画プレーヤーの状態
#[derive(Debug, Clone, Copy, PartialEq)]
enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

/// 動画プレーヤー
struct VideoPlayer {
    video_path: PathBuf,
    state: PlayerState,
    current_frame: Arc<Mutex<Option<Vec<u8>>>>,
    frame_sender: mpsc::Sender<Vec<u8>>,
    frame_receiver: mpsc::Receiver<Vec<u8>>,
    total_frames: i64,
    current_frame_index: i64,
    width: u32,
    height: u32,
}

impl VideoPlayer {
    fn new(video_path: PathBuf) -> Result<Self> {
        // FFmpegを初期化
        ffmpeg::init()?;

        // 動画ファイルを開く
        let input = ffmpeg::format::input(&video_path)?;
        let video_stream = input
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or_else(|| anyhow::anyhow!("No video stream found"))?;

        // フレーム数を取得
        let total_frames = video_stream.frames();

        // 動画のサイズを取得
        let decoder = video_stream.codec().decoder().video().unwrap();
        let width = decoder.width() as u32;
        let height = decoder.height() as u32;

        let (frame_sender, frame_receiver) = tokio::sync::mpsc::channel(30);
        let current_frame = Arc::new(Mutex::new(Option::<Vec<u8>>::None));

        let mut player = VideoPlayer {
            video_path,
            state: PlayerState::Stopped,
            current_frame,
            frame_sender,
            frame_receiver,
            width,
            height,
            current_frame_index: 0,
            total_frames,
        };

        // フレーム読み込みスレッドを開始
        player.start_playback();

        Ok(player)
    }

    fn toggle_playback(&mut self) {
        match self.state {
            PlayerState::Playing => {
                self.state = PlayerState::Paused;
            }
            PlayerState::Paused | PlayerState::Stopped => {
                self.state = PlayerState::Playing;
                self.start_playback();
            }
        }
    }

    fn start_playback(&mut self) {
        let video_path = self.video_path.clone();
        let frame_sender = self.frame_sender.clone();

        std::thread::spawn(move || {
            let mut input = ffmpeg::format::input(&video_path).unwrap();
            let stream_index = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .unwrap()
                .index();
            let mut decoder = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .unwrap()
                .codec()
                .decoder()
                .video()
                .unwrap();

            let mut scaler = ffmpeg::software::scaling::Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                ffmpeg::format::Pixel::RGB24,
                decoder.width(),
                decoder.height(),
                ffmpeg::software::scaling::Flags::BILINEAR,
            )
            .unwrap();

            for (stream, packet) in input.packets() {
                if stream.index() == stream_index {
                    decoder.send_packet(&packet).unwrap();
                    let mut decoded = ffmpeg::frame::Video::empty();
                    while decoder.receive_frame(&mut decoded).is_ok() {
                        let mut rgb_frame = ffmpeg::frame::Video::empty();
                        scaler.run(&decoded, &mut rgb_frame).unwrap();
                        frame_sender
                            .blocking_send(rgb_frame.data(0).to_vec())
                            .unwrap();
                    }
                }
            }
        });
    }

    fn next_frame(&mut self) {
        if self.current_frame_index < self.total_frames - 1 {
            self.current_frame_index += 1;
            self.update_frame();
        }
    }

    fn previous_frame(&mut self) {
        if self.current_frame_index > 0 {
            self.current_frame_index -= 1;
            self.update_frame();
        }
    }

    fn update_frame(&mut self) {
        let mut input = ffmpeg::format::input(&self.video_path).unwrap();
        let stream_index = input
            .streams()
            .best(ffmpeg::media::Type::Video)
            .unwrap()
            .index();
        let mut decoder = input
            .streams()
            .best(ffmpeg::media::Type::Video)
            .unwrap()
            .codec()
            .decoder()
            .video()
            .unwrap();
        let width = decoder.width() as u32;
        let height = decoder.height() as u32;

        let target_ts = (self.current_frame_index as f64 * 1000.0) as i64;
        input.seek(target_ts, ..target_ts).unwrap();

        for (stream, packet) in input.packets() {
            if stream.index() == stream_index {
                decoder.send_packet(&packet).unwrap();
                let mut decoded = ffmpeg::frame::Video::empty();
                if decoder.receive_frame(&mut decoded).is_ok() {
                    let mut scaler = ffmpeg::software::scaling::Context::get(
                        decoder.format(),
                        decoder.width(),
                        decoder.height(),
                        ffmpeg::format::Pixel::RGB24,
                        decoder.width(),
                        decoder.height(),
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )
                    .unwrap();
                    let mut rgb_frame = ffmpeg::frame::Video::empty();
                    scaler.run(&decoded, &mut rgb_frame).unwrap();
                    *self.current_frame.lock().unwrap() = Some(rgb_frame.data(0).to_vec());
                    break;
                }
            }
        }
    }

    fn display_frame(&self) {
        if let Some(frame) = self.current_frame.lock().unwrap().as_ref() {
            // フレームをPNGファイルとして保存
            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(self.width, self.height, frame.clone())
                .unwrap();
            img.save(format!("frame_{}.png", self.current_frame_index + 1))
                .unwrap();
            println!("フレーム {} を保存しました", self.current_frame_index + 1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // コマンドライン引数から動画ファイルのパスを取得
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("使用方法: {} <動画ファイルのパス>", args[0]);
        std::process::exit(1);
    }

    let video_path = PathBuf::from(&args[1]);
    let mut player = VideoPlayer::new(video_path)?;

    println!("EDV Video Player");
    println!("コマンド:");
    println!("  p: 再生/一時停止");
    println!("  n: 次のフレーム");
    println!("  b: 前のフレーム");
    println!("  q: 終了");

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        match input.trim() {
            "p" => player.toggle_playback(),
            "n" => player.next_frame(),
            "b" => player.previous_frame(),
            "q" => break,
            _ => println!("無効なコマンドです"),
        }

        player.display_frame();
    }

    Ok(())
}
