// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Offscreen buffer module for terminal rendering and VT100/ANSI terminal emulation.
//!
//! This module provides a comprehensive terminal screen buffer implementation that works
//! seamlessly with both the render pipeline and VT100/ANSI escape sequences. The buffer
//! is organized as a flexible grid of pixel characters with full support for
//! variable-width characters (like emojis and Unicode).
//!
//! # Architecture Overview
//!
//! The offscreen buffer serves as the central data structure for terminal emulation,
//! bridging the gap between raw terminal output and visual rendering:
//!
//! ```text
//! ╭─────────────────╮    ╭──────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master   │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream)│    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰──────────────╯    ╰─────────────────╯    │  buffer)     │
//! │                                                                  ╰──────────────╯
//! │                                           ╭─────────────────╮           │
//! │                                           │ RenderPipeline  │◀──────────╯
//! │                                           │ paint()         │
//! ╰───────────────────────────────────────────▶ Terminal Output │
//!                                             ╰─────────────────╯
//! ```
//!
//! # Dual Integration Points
//!
//! The offscreen buffer is designed to work seamlessly with two major subsystems:
//!
//! ## 1. ANSI/VT100 Terminal Emulation
//!
//! - **Parser Integration**: Processes escape sequences via [`vt100_ansi_impl`]
//!   implementations
//! - **State Management**: Maintains cursor position, character sets, scrolling regions
//! - **Protocol Compliance**: Full VT100 specification compliance with conformance tests
//! - **Character Handling**: Supports both ASCII and DEC graphics character sets
//!
//! ## 2. Render Pipeline Integration
//!
//! - **Visual Rendering**: Direct integration with [`RenderPipeline::paint()`]
//! - **Styling Support**: Rich text styling with [`TuiStyle`] for colors and attributes
//! - **Performance**: Efficient diff-based rendering to minimize screen updates
//! - **Multi-width Characters**: Proper handling of emoji and Unicode display widths
//!
//! # Grid Representation & Variable-Width Characters
//!
//! The buffer implements a sophisticated grid system that handles the complexity of
//! modern terminal content:
//!
//! ```text
//! Terminal Grid (cell-by-cell mapping):
//! ┌───┬───┬───┬───┬───┐
//! │ H │ e │ 😃│ ∅ │ ! │  ← Row 0
//! └───┴───┴───┴───┴───┘
//!       │   │   │
//!       │   │   └─ PixelChar::Void (placeholder for wide char)
//!       │   └─ PixelChar::PlainText { display_char: '😃', width: 2 }
//!       └─ PixelChar::PlainText { display_char: 'e', width: 1 }
//! ```
//!
//! **Key Design Principles:**
//! - **Cell Mapping**: Each grid position maps directly to a terminal screen position
//! - **Wide Character Handling**: Multi-width characters use [`PixelChar::Void`]
//!   placeholders
//! - **Rendering Integrity**: Void cells ensure proper visual alignment and cursor
//!   positioning
//! - **Unicode Support**: Full support for emoji, combining characters, and international
//!   text
//!
//! # VT100 Implementation Architecture - 1:1 Mapping
//!
//! The buffer's VT100 support follows a clean architectural pattern with perfect file
//! mapping:
//!
//! ```text
//! vt_100_ansi_parser/operations/     offscreen_buffer/vt100_ansi_impl/
//! ├── char_ops.rs         →         ├── char_ops.rs         (print_char, ICH, DCH, ECH)
//! ├── control_ops.rs      →         ├── control_ops.rs      (BS, TAB, LF, CR)
//! ├── cursor_ops.rs       →         ├── cursor_ops.rs       (movement, positioning)
//! ├── line_ops.rs         →         ├── line_ops.rs         (insert/delete lines)
//! ├── scroll_ops.rs       →         ├── scroll_ops.rs       (scrolling, regions)
//! ├── terminal_ops.rs     →         ├── terminal_ops.rs     (reset, clear, charset)
//! └── bounds_check.rs     →         └── bounds_check.rs     (safety utilities)
//! ```
//!
//! This 1:1 mapping provides:
//! - **Predictable Navigation**: Easy to find implementation for any ANSI operation
//! - **Clear Separation**: Parser logic separate from buffer implementation
//! - **Comprehensive Testing**: Each implementation file has full unit test coverage
//!
//! [`RenderPipeline::paint()`]: crate::RenderPipeline::paint
//! [`TuiStyle`]: crate::TuiStyle
//! [`PixelChar::Void`]: PixelChar::Void

// Attach.
pub mod diff_chunks;
pub mod ofs_buf_bulk_ops;
pub mod ofs_buf_char_ops;
pub mod ofs_buf_core;
pub mod ofs_buf_line_level_ops;
pub mod pixel_char;
pub mod pixel_char_line;
pub mod pixel_char_lines;
pub mod vt100_ansi_impl;

// Re-export all implementations.
pub use diff_chunks::*;
pub use ofs_buf_core::*;
pub use pixel_char::*;
pub use pixel_char_line::*;
pub use pixel_char_lines::*;

// Test fixtures (only available during testing).
#[cfg(test)]
pub mod test_fixtures_ofs_buf;
