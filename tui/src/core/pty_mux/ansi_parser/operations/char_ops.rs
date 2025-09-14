// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character insertion, deletion, and erasure operations.
//!
//! # CSI Sequence Architecture
//!
//! ```text
//! Application sends "ESC[2P" (delete 2 chars)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [THIS METHOD]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount};

/// Handle DCH (Delete Character) - delete n characters at cursor position.
/// Characters to the right of cursor shift left.
/// Blank characters are inserted at the end of the line.
///
/// Example - Deleting 2 characters at cursor position
///
/// ```text
/// Before:
///           ╭────── max_width=10 (1-based) ──────╮
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │ c │ d │ E │ F │ G │ H │ I │ J │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// After DCH 2:
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │ E │ F │ G │ H │ I │ J │   │   │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// Result: C and D deleted, E-F-G shifted left, blanks filled at end
/// ```
pub fn delete_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);
    let at = /* 0-based */ performer.ofs_buf.my_pos;
    let max_width = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Use dedicated DCH method to delete characters at cursor.
    performer
        .ofs_buf
        .delete_chars_at_cursor(at, how_many, max_width);
}

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
/// Characters to the right of cursor shift right.
/// Characters shifted beyond the right margin are lost.
///
/// Example - Inserting 2 blank characters at cursor position
///
/// ```text
/// Before:
///           ╭────── max_width=10 (1-based) ─────╮
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// After ICH 2:
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │   │   │ C │ D │ E │ F │ G │ H │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// Result: 2 blanks inserted, C-D-E-F-G-H shifted right, I-J lost beyond margin
/// ```
pub fn insert_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);
    let at = /* 0-based */ performer.ofs_buf.my_pos;
    let max_width = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Use dedicated ICH method to insert characters at cursor.
    performer
        .ofs_buf
        .insert_chars_at_cursor(at, how_many, max_width);
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs.
/// This is different from DCH which shifts characters left.
///
/// Example - Erasing 3 characters at cursor position
///
/// ```text
/// Before:
///           ╭────── max_width=10 (1-based) ──────╮
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// After ECH 3:
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │   │   │   │ F │ G │ H │ I │ J │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// Result: C, D, E replaced with blanks, F-G-H-I-J remain in place (no shifting)
/// ```
pub fn erase_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);
    let at = /* 0-based */ performer.ofs_buf.my_pos;
    let max_width = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Use dedicated ECH method to erase characters at cursor.
    performer
        .ofs_buf
        .erase_chars_at_cursor(at, how_many, max_width);
}
