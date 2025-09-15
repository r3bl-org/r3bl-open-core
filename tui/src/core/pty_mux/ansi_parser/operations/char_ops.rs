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
/// Characters to the right of cursor shift left, blanks are inserted at line end.
/// See `OffscreenBuffer::delete_chars_at_cursor` for detailed behavior and examples.
pub fn delete_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);

    // Use dedicated DCH method to delete characters at cursor.
    performer
        .ofs_buf
        .delete_chars_at_cursor(how_many);
}

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
/// Characters to the right of cursor shift right, characters beyond margin are lost.
/// See `OffscreenBuffer::insert_chars_at_cursor` for detailed behavior and examples.
pub fn insert_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);

    // Use dedicated ICH method to insert characters at cursor.
    performer
        .ofs_buf
        .insert_chars_at_cursor(how_many);
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs (unlike DCH).
/// See `OffscreenBuffer::erase_chars_at_cursor` for detailed behavior and examples.
pub fn erase_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);

    // Use dedicated ECH method to erase characters at cursor.
    performer
        .ofs_buf
        .erase_chars_at_cursor(how_many);
}
