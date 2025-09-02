// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line insertion and deletion operations.

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   param_utils::ParamsExt};
use crate::PixelChar;

/// Handle IL (Insert Line) - insert n blank lines at cursor position.
/// Lines below cursor and within scroll region shift down.
/// Lines scrolled off the bottom are lost.
pub fn insert_lines(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    let cursor_row = processor.ofs_buf.my_pos.row_index.as_usize();
    
    for _ in 0..n {
        insert_line_at(processor, cursor_row);
    }
    
    tracing::trace!("CSI {}L (IL): Inserted {} lines at row {}", n, n, cursor_row);
}

/// Handle DL (Delete Line) - delete n lines starting at cursor position.
/// Lines below cursor and within scroll region shift up.
/// Blank lines are added at the bottom of the scroll region.
pub fn delete_lines(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    let cursor_row = processor.ofs_buf.my_pos.row_index.as_usize();
    
    for _ in 0..n {
        delete_line_at(processor, cursor_row);
    }
    
    tracing::trace!("CSI {}M (DL): Deleted {} lines at row {}", n, n, cursor_row);
}

/// Insert a single blank line at the specified row.
/// Lines below shift down within the scroll region.
/// The bottom line of the scroll region is lost.
fn insert_line_at(processor: &mut AnsiToBufferProcessor, row: usize) {
    let max_row = processor.ofs_buf.window_size.row_height.as_usize();
    
    // Get scroll region boundaries (1-based to 0-based conversion)
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based)
        .map_or(0, |row| row.as_usize());
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based)
        .map_or(max_row.saturating_sub(1), |row| row.as_usize());
    
    // Only operate within scroll region and if cursor is within region
    if row < scroll_top || row > scroll_bottom {
        return;
    }
    
    // Shift lines down within the scroll region, from bottom to insertion point
    for shift_row in (row + 1..=scroll_bottom).rev() {
        if shift_row > 0 {
            processor.ofs_buf.buffer[shift_row] = processor.ofs_buf.buffer[shift_row - 1].clone();
        }
    }
    
    // Clear the newly inserted line
    clear_line(processor, row);
}

/// Delete a single line at the specified row.
/// Lines below shift up within the scroll region.
/// A blank line is added at the bottom of the scroll region.
fn delete_line_at(processor: &mut AnsiToBufferProcessor, row: usize) {
    let max_row = processor.ofs_buf.window_size.row_height.as_usize();
    
    // Get scroll region boundaries (1-based to 0-based conversion)
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based)
        .map_or(0, |row| row.as_usize());
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based)
        .map_or(max_row.saturating_sub(1), |row| row.as_usize());
    
    // Only operate within scroll region and if cursor is within region
    if row < scroll_top || row > scroll_bottom {
        return;
    }
    
    // Shift lines up within the scroll region, from deletion point to bottom
    for shift_row in row..scroll_bottom {
        processor.ofs_buf.buffer[shift_row] = processor.ofs_buf.buffer[shift_row + 1].clone();
    }
    
    // Clear the bottom line of the scroll region (new blank line)
    clear_line(processor, scroll_bottom);
}

/// Clear a line by filling it with blanks.
fn clear_line(processor: &mut AnsiToBufferProcessor, row: usize) {
    let col_width = processor.ofs_buf.window_size.col_width.as_usize();
    for col in 0..col_width {
        processor.ofs_buf.buffer[row][col] = PixelChar::Spacer;
    }
}