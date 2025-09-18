// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scrolling operations.
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭──────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master   │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream)│    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰──────────────╯    ╰─────────────────╯    │  buffer)     │
//!                                                     │             ╰──────────────╯
//!                                                     ▼
//!                                            ╭─────────────────╮
//!                                            │ Perform Trait   │
//!                                            │ Implementation  │
//!                                            ╰─────────────────╯
//! ```
//!
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[3S" (scroll up 3 lines)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H) ╭───────────╮
//!       - scroll_ops:: for scrolling (S,T) <--- │THIS MODULE│
//!       - sgr_ops:: for styling (m)             ╰───────────╯
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount};

/// Move cursor down one line, scrolling the buffer if at bottom.
/// Implements the ESC D (IND) escape sequence.
/// Respects DECSTBM scroll region margins.
/// See [`crate::OffscreenBuffer::index_down`] for detailed behavior and examples.
pub fn index_down(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.index_down();
}

/// Move cursor up one line, scrolling the buffer if at top.
/// Implements the ESC M (RI) escape sequence.
/// Respects DECSTBM scroll region margins.
/// See [`crate::OffscreenBuffer::reverse_index_up`] for detailed behavior and examples.
pub fn reverse_index_up(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.reverse_index_up();
}

/// Scroll buffer content up by one line (for ESC D at bottom).
/// The top line is lost, and a new empty line appears at bottom.
/// Respects DECSTBM scroll region margins.
/// See [`crate::OffscreenBuffer::scroll_buffer_up`] for detailed behavior and examples.
pub fn scroll_buffer_up(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.scroll_buffer_up();
}

/// Scroll buffer content down by one line (for ESC M at top).
/// The bottom line is lost, and a new empty line appears at top.
/// Respects DECSTBM scroll region margins.
/// See [`crate::OffscreenBuffer::scroll_buffer_down`] for detailed behavior and examples.
pub fn scroll_buffer_down(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.scroll_buffer_down();
}

/// Handle SU (Scroll Up) - scroll display up by n lines.
/// See [`crate::OffscreenBuffer::scroll_up`] for detailed behavior and examples.
pub fn scroll_up(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    performer.ofs_buf.scroll_up(how_many);
}

/// Handle SD (Scroll Down) - scroll display down by n lines.
/// See [`crate::OffscreenBuffer::scroll_down`] for detailed behavior and examples.
pub fn scroll_down(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    performer.ofs_buf.scroll_down(how_many);
}
