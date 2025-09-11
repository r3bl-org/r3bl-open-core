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
use crate::{CharacterSet, Pos};

/// Clear all buffer content.
fn clear_buffer(performer: &mut AnsiToOfsBufPerformer) { performer.ofs_buf.clear(); }

/// Reset all SGR attributes to default state.
fn reset_sgr_attributes(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.ansi_parser_support.current_style = None;
    performer.ofs_buf.ansi_parser_support.attribs.bold = None;
    performer.ofs_buf.ansi_parser_support.attribs.dim = None;
    performer.ofs_buf.ansi_parser_support.attribs.italic = None;
    performer.ofs_buf.ansi_parser_support.attribs.underline = None;
    performer.ofs_buf.ansi_parser_support.attribs.blink = None;
    performer.ofs_buf.ansi_parser_support.attribs.reverse = None;
    performer.ofs_buf.ansi_parser_support.attribs.hidden = None;
    performer.ofs_buf.ansi_parser_support.attribs.strikethrough = None;
    performer.ofs_buf.ansi_parser_support.fg_color = None;
    performer.ofs_buf.ansi_parser_support.bg_color = None;
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
