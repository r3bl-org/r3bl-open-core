// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character insertion, deletion, and erasure operations.

use std::cmp::min;

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   protocols::csi_codes::MovementCount};
use crate::{PixelChar, len, BoundsCheck, BoundsStatus};

/// Handle DCH (Delete Character) - delete n characters at cursor position.
/// Characters to the right of cursor shift left.
/// Blank characters are inserted at the end of the line.
pub fn delete_chars(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let chars_to_delete = len(MovementCount::from(params).as_u16());

    let current_row = processor.ofs_buf.my_pos.row_index;
    let current_col = processor.ofs_buf.my_pos.col_index;
    let max_col = processor.ofs_buf.window_size.col_width;

    // Nothing to delete if cursor is at or beyond right margin
    if current_col.check_overflows(max_col) == BoundsStatus::Overflowed {
        return;
    }

    // Calculate how many characters we can actually delete
    let actual_chars_to_delete = min(
        chars_to_delete.as_usize(),
        max_col.as_usize() - current_col.as_usize(),
    );

    // Shift characters left to fill the gap using copy_within
    let source_range =
        current_col.as_usize() + actual_chars_to_delete..max_col.as_usize();
    processor.ofs_buf.buffer[current_row.as_usize()]
        .copy_within(source_range, current_col.as_usize());

    // Fill the end of the line with blank characters
    let blank_range = max_col.as_usize() - actual_chars_to_delete..max_col.as_usize();
    processor.ofs_buf.buffer[current_row.as_usize()][blank_range].fill(PixelChar::Spacer);
}

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
/// Characters to the right of cursor shift right.
/// Characters shifted beyond the right margin are lost.
pub fn insert_chars(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let chars_to_insert = len(MovementCount::from(params).as_u16());
    let current_row = processor.ofs_buf.my_pos.row_index;
    let current_col = processor.ofs_buf.my_pos.col_index;
    let max_col = processor.ofs_buf.window_size.col_width;

    // Nothing to insert if cursor is at or beyond right margin
    if current_col.check_overflows(max_col) == BoundsStatus::Overflowed {
        return;
    }

    // Calculate how many characters we can actually insert
    let actual_chars_to_insert = min(
        chars_to_insert.as_usize(),
        max_col.as_usize() - current_col.as_usize(),
    );

    // Shift characters right using copy_within
    let destination_col = current_col.as_usize() + actual_chars_to_insert;
    let source_range =
        current_col.as_usize()..max_col.as_usize() - actual_chars_to_insert;
    processor.ofs_buf.buffer[current_row.as_usize()]
        .copy_within(source_range, destination_col);

    // Insert blank characters at cursor position
    let insert_range = current_col.as_usize()..destination_col;
    processor.ofs_buf.buffer[current_row.as_usize()][insert_range]
        .fill(PixelChar::Spacer);
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs.
/// This is different from DCH which shifts characters left.
pub fn erase_chars(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let chars_to_erase = len(MovementCount::from(params).as_u16());
    let current_row = processor.ofs_buf.my_pos.row_index;
    let current_col = processor.ofs_buf.my_pos.col_index;
    let max_col = processor.ofs_buf.window_size.col_width;

    // Nothing to erase if cursor is at or beyond right margin
    if current_col.check_overflows(max_col) == BoundsStatus::Overflowed {
        return;
    }

    // Calculate how many characters we can actually erase
    let actual_chars_to_erase = min(
        chars_to_erase.as_usize(),
        max_col.as_usize() - current_col.as_usize(),
    );

    // Replace characters with blanks (no shifting)
    let erase_range =
        current_col.as_usize()..current_col.as_usize() + actual_chars_to_erase;
    processor.ofs_buf.buffer[current_row.as_usize()][erase_range].fill(PixelChar::Spacer);
}
