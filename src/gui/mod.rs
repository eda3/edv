// GUI module for the EDV application
//
// This module provides a simple GUI functionality without using external GUI libraries,
// leveraging FFmpeg's capabilities to create a basic video player interface.

pub mod commands;
pub mod frame_player;

// Re-export the main types for convenience
pub use commands::GuiPlayCommand;
pub use frame_player::FramePlayer;
