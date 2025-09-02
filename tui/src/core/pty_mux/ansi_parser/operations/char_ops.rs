// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character insertion, deletion, and erasure operations.

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   param_utils::ParamsExt};
use crate::PixelChar;

/// Handle DCH (Delete Character) - delete n characters at cursor position.
/// Characters to the right of cursor shift left.
/// Blank characters are inserted at the end of the line.
pub fn delete_chars(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    let cursor_row = processor.ofs_buf.my_pos.row_index.as_usize();
    let cursor_col = processor.ofs_buf.my_pos.col_index.as_usize();
    let col_width = processor.ofs_buf.window_size.col_width.as_usize();
    
    // Nothing to delete if cursor is at or beyond right margin
    if cursor_col >= col_width {
        return;
    }
    
    // Calculate how many characters we can actually delete
    let chars_to_delete = std::cmp::min(n as usize, col_width - cursor_col);
    
    // Shift characters left
    for col in cursor_col..(col_width - chars_to_delete) {
        if col + chars_to_delete < col_width {
            processor.ofs_buf.buffer[cursor_row][col] = 
                processor.ofs_buf.buffer[cursor_row][col + chars_to_delete].clone();
        }
    }
    
    // Fill the end of the line with blank characters
    for col in (col_width - chars_to_delete)..col_width {
        processor.ofs_buf.buffer[cursor_row][col] = PixelChar::Spacer;
    }
    
    tracing::trace!("CSI {}P (DCH): Deleted {} characters at row {}, col {}", 
                   n, chars_to_delete, cursor_row, cursor_col);
}

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
/// Characters to the right of cursor shift right.
/// Characters shifted beyond the right margin are lost.
pub fn insert_chars(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    let cursor_row = processor.ofs_buf.my_pos.row_index.as_usize();
    let cursor_col = processor.ofs_buf.my_pos.col_index.as_usize();
    let col_width = processor.ofs_buf.window_size.col_width.as_usize();
    
    // Nothing to insert if cursor is at or beyond right margin
    if cursor_col >= col_width {
        return;
    }
    
    // Calculate how many characters we can actually insert
    let chars_to_insert = std::cmp::min(n as usize, col_width - cursor_col);
    
    // Shift characters right (work backwards to avoid overwriting)
    for col in (cursor_col + chars_to_insert..col_width).rev() {
        if col >= chars_to_insert {
            processor.ofs_buf.buffer[cursor_row][col] = 
                processor.ofs_buf.buffer[cursor_row][col - chars_to_insert].clone();
        }
    }
    
    // Insert blank characters at cursor position
    for col in cursor_col..(cursor_col + chars_to_insert) {
        processor.ofs_buf.buffer[cursor_row][col] = PixelChar::Spacer;
    }
    
    tracing::trace!("CSI {}@ (ICH): Inserted {} characters at row {}, col {}", 
                   n, chars_to_insert, cursor_row, cursor_col);
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs.
/// This is different from DCH which shifts characters left.
pub fn erase_chars(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    let cursor_row = processor.ofs_buf.my_pos.row_index.as_usize();
    let cursor_col = processor.ofs_buf.my_pos.col_index.as_usize();
    let col_width = processor.ofs_buf.window_size.col_width.as_usize();
    
    // Nothing to erase if cursor is at or beyond right margin
    if cursor_col >= col_width {
        return;
    }
    
    // Calculate how many characters we can actually erase
    let chars_to_erase = std::cmp::min(n as usize, col_width - cursor_col);
    
    // Replace characters with blanks (no shifting)
    for col in cursor_col..(cursor_col + chars_to_erase) {
        processor.ofs_buf.buffer[cursor_row][col] = PixelChar::Spacer;
    }
    
    tracing::trace!("CSI {}X (ECH): Erased {} characters at row {}, col {}", 
                   n, chars_to_erase, cursor_row, cursor_col);
}