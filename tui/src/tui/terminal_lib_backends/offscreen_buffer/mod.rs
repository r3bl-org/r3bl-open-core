// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Offscreen buffer module for terminal rendering
//!
//! This module provides a flexible representation of a terminal screen buffer that works
//! with both the render pipeline and ANSI escape sequences. The buffer is organized as
//! a grid of pixel characters with support for variable-width characters (like emojis).

// Attach.
pub mod diff_chunks;
pub mod ofs_buf_bulk_ops;
pub mod ofs_buf_char_ops;
pub mod ofs_buf_core;
pub mod ofs_buf_line_level_ops;
pub mod ofs_buf_shifting_ops;
pub mod pixel_char;
pub mod pixel_char_line;
pub mod pixel_char_lines;

// Re-export all implementations.
pub use diff_chunks::*;
pub use ofs_buf_core::*;
pub use pixel_char::*;
pub use pixel_char_line::*;
pub use pixel_char_lines::*;

// Test fixtures (only available during testing).
#[cfg(test)]
pub mod ofs_buf_test_fixtures;
