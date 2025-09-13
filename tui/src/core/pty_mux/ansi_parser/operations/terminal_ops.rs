// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal state operations.
//!
//! # CSI Sequence Architecture
//!
//! ```text
//! Application sends "ESC c" (reset terminal)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC char pattern)
//!         ↓
//!     esc_dispatch() [THIS METHOD]
//!         ↓
//!     Handle terminal state operations:
//!       - reset_terminal() for ESC c (RIS)
//!       - save/restore cursor for ESC 7/8
//!       - character set selection
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::{CharacterSet, Pos, TuiStyle};

/// Clear all buffer content.
fn clear_buffer(performer: &mut AnsiToOfsBufPerformer) { performer.ofs_buf.clear(); }

/// Reset all SGR attributes to default state.
fn reset_sgr_attributes(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.ansi_parser_support.current_style = TuiStyle::default();
}

/// Reset terminal to initial state (ESC c).
/// Clears the buffer, resets cursor, and clears saved state.
/// Clears DECSTBM scroll region margins.
pub fn reset_terminal(performer: &mut AnsiToOfsBufPerformer) {
    clear_buffer(performer);

    // Reset cursor to home position.
    performer.ofs_buf.my_pos = Pos::default();

    // Clear saved cursor state.
    performer
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore = None;

    // Reset to ASCII character set.
    performer.ofs_buf.ansi_parser_support.character_set = CharacterSet::Ascii;

    // Clear DECSTBM scroll region margins.
    performer.ofs_buf.ansi_parser_support.scroll_region_top = None;
    performer.ofs_buf.ansi_parser_support.scroll_region_bottom = None;

    // Clear any SGR attributes.
    reset_sgr_attributes(performer);
}
