// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal state operations for ESC sequences.
//!
//! # ESC Sequence Architecture
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
//!     esc_dispatch() [calls functions in this module]
//!         ↓
//!     terminal_ops functions:
//!       - reset_terminal() for ESC c (RIS)
//!       - select_ascii_character_set() for ESC ( B
//!       - select_dec_graphics_character_set() for ESC ( 0
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! Note: Cursor save/restore ESC sequences (ESC 7/8) are handled by `cursor_ops`
//! functions to maintain consistency with CSI equivalents (CSI s/u).

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::{Pos, TuiStyle};

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
    performer.ofs_buf.cursor_pos = Pos::default();

    // Clear saved cursor state.
    performer
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore = None;

    // Reset to ASCII character set.
    select_ascii_character_set(performer);

    // Clear DECSTBM scroll region margins.
    performer.ofs_buf.ansi_parser_support.scroll_region_top = None;
    performer.ofs_buf.ansi_parser_support.scroll_region_bottom = None;

    // Clear any SGR attributes.
    reset_sgr_attributes(performer);
}

/// Select ASCII character set (ESC ( B).
/// Switches to normal ASCII character set for standard text rendering.
pub fn select_ascii_character_set(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.select_ascii_character_set();
}

/// Select DEC Special Graphics character set (ESC ( 0).
/// Switches to DEC Special Graphics character set for box-drawing characters.
pub fn select_dec_graphics_character_set(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.select_dec_graphics_character_set();
}
