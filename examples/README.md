# EDV Sample Projects Collection

This directory contains sample projects using the EDV library. These samples demonstrate the key features of EDV through practical code examples.

## Sample List

### 1. [multi_track_demo.rs](./multi_track_demo.rs)

A basic multi-track editing sample. This sample demonstrates:
- Creating multiple video and audio tracks
- Positioning clips and adjusting timing
- Setting opacity using keyframes

### 2. [multi_track_keyframe_demo.rs](./multi_track_keyframe_demo.rs)

A multi-track editing sample with keyframe animations. This sample demonstrates:
- Creating multiple video and audio tracks
- Fade in/fade out opacity animations
- Scale (size) animations
- Position animations (X-coordinate movement)
- Using easing functions

### 3. [advanced_multi_track_demo.rs](./advanced_multi_track_demo.rs)

A sample demonstrating more advanced multi-track editing techniques. This sample showcases:
- Multiple overlapping video layers (background, main, overlay)
- Multiple audio tracks (main, sound effects)
- Setting track mute and lock states
- Complex keyframe animations
- Audio fade in/fade out

## How to Run

To run the samples, you first need to prepare a video file in the `test_media` directory:

```bash
# Create test_media directory
mkdir -p test_media

# Place a video file (use your own video or download a sample)
# Example: Downloading a sample video
wget https://sample-videos.com/video123/mp4/720/big_buck_bunny_720p_1mb.mp4 -O test_media/sozai.mp4
```

### Running in a Normal Environment

```bash
# Create output directory
mkdir -p output

# Run the samples
cargo run --example multi_track_demo
cargo run --example multi_track_keyframe_demo
cargo run --example advanced_multi_track_demo
```

### Running in a WSL Environment

In WSL environments, temporary directory issues may occur. You can work around this with the following method:

```bash
# Create a temporary directory
mkdir -p output/temp
chmod -R 1777 output

# Run with the TMPDIR environment variable set
TMPDIR=$(pwd)/output/temp cargo run --example multi_track_demo
TMPDIR=$(pwd)/output/temp cargo run --example multi_track_keyframe_demo
TMPDIR=$(pwd)/output/temp cargo run --example advanced_multi_track_demo
```

## Output Results

When the samples run successfully, the following files will be generated in the `output` directory:

- `multi_track_output.mp4` - Basic multi-track editing output
- `keyframe_animation_output.mp4` - Output with keyframe animations
- `advanced_multi_track_output.mp4` - Advanced multi-track editing output

## Notes

- Make sure FFmpeg is installed on your system
- When running in a WSL environment, setting the TMPDIR environment variable is necessary
- The samples use the same video file as multiple assets, but in real projects, it's common to use multiple different media files 