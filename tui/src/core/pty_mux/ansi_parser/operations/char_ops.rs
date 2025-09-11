// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character insertion, deletion, and erasure operations.

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount};
use crate::{LengthMarker, PixelChar, len};

/// Handle DCH (Delete Character) - delete n characters at cursor position.
/// Characters to the right of cursor shift left.
/// Blank characters are inserted at the end of the line.
///
/// Example - Deleting 2 characters at cursor position
///
/// ```text
/// Before:
///           ╭────── max_width=10 (1-based) ─────╮
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │ C │ D │ E │ F │ G │   │   │   │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// After DCH 2:
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:    │ A │ B │ E │ F │ G │   │   │   │   │   │
///         └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                   ╰ cursor (col 2, 0-based)
///
/// Result: C and D deleted, E-F-G shifted left, blanks filled at end
/// ```
pub fn delete_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_length(params);
    let cur_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;
    let cur_col = /* 0-based */ performer.ofs_buf.my_pos.col_index;
    let max_width = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Nothing to delete if cursor is at or beyond right margin
    if max_width.is_overflowed_by(cur_col) {
        return;
    }

    // Calculate how many characters we can actually delete
    let how_many_clamped = how_many.clamp_to(max_width.remaining_from(cur_col));

    // Shift characters left to fill the gap using copy_within
    performer.ofs_buf.copy_chars_within_line(
        cur_row,
        {
            let start = cur_col + how_many_clamped;
            let end = max_width.convert_to_index() + len(1);
            start..end
        },
        cur_col,
    );

    // Fill the end of the line with blank characters
    performer.ofs_buf.fill_char_range(
        cur_row,
        {
            let start = max_width.convert_to_index() - how_many_clamped + len(1);
            let end = max_width.convert_to_index() + len(1);
            start..end
        },
        PixelChar::Spacer,
    );
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
    let cur_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;
    let cur_col = /* 0-based */ performer.ofs_buf.my_pos.col_index;
    let max_width = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Nothing to insert if cursor is at or beyond right margin
    if max_width.is_overflowed_by(cur_col) {
        return;
    }

    // Calculate how many characters we can actually insert
    let how_many_clamped = how_many.clamp_to(max_width.remaining_from(cur_col));

    // Use dedicated ICH method to insert characters at cursor
    performer.ofs_buf.insert_chars_at_cursor(
        cur_row,
        cur_col,
        how_many_clamped,
        max_width,
    );
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs.
/// This is different from DCH which shifts characters left.
///
/// Example - Erasing 3 characters at cursor position
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
    let cur_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;
    let cur_col = /* 0-based */ performer.ofs_buf.my_pos.col_index;
    let max_width = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Nothing to erase if cursor is at or beyond right margin
    if max_width.is_overflowed_by(cur_col) {
        return;
    }

    // Calculate how many characters we can actually erase
    let how_many_clamped = how_many.clamp_to(max_width.remaining_from(cur_col));

    // Use fill_char_range to erase characters
    performer.ofs_buf.fill_char_range(
        cur_row,
        {
            let start = cur_col;
            let end = cur_col + how_many_clamped;
            start..end
        },
        PixelChar::Spacer,
    );
}
