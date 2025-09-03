// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line insertion and deletion operations.

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   protocols::csi_codes::MovementCount, term_units::TermRow};
use crate::PixelChar;

/// Handle IL (Insert Line) - insert n blank lines at cursor position.
/// Lines below cursor and within scroll region shift down.
/// Lines scrolled off the bottom are lost.
pub fn insert_lines(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let lines_to_insert = MovementCount::from(params).as_u16();
    let current_row = processor.ofs_buf.my_pos.row_index;

    for _ in 0..lines_to_insert {
        insert_line_at(processor, current_row.as_usize());
    }
}

/// Handle DL (Delete Line) - delete n lines starting at cursor position.
/// Lines below cursor and within scroll region shift up.
/// Blank lines are added at the bottom of the scroll region.
pub fn delete_lines(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let lines_to_delete = MovementCount::from(params).as_u16();
    let current_row = processor.ofs_buf.my_pos.row_index;

    for _ in 0..lines_to_delete {
        delete_line_at(processor, current_row.as_usize());
    }
}

/// Insert a single blank line at the specified row.
/// Lines below shift down within the scroll region.
/// The bottom line of the scroll region is lost.
fn insert_line_at(processor: &mut AnsiToBufferProcessor, row: usize) {
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ 0, /* Some */ |row| row.as_usize());

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ max_row.convert_to_row_index().as_usize(), /* Some */ |row| row.as_usize());

    // Only operate within scroll region and if cursor is within region.
    if row < scroll_top || row > scroll_bottom {
        return;
    }

    // Shift lines down within the scroll region, from bottom to insertion point.
    for shift_row in (row + 1..=scroll_bottom).rev() {
        if shift_row > 0 {
            processor.ofs_buf.buffer[shift_row] =
                processor.ofs_buf.buffer[shift_row - 1].clone();
        }
    }

    // Clear the newly inserted line.
    clear_line(processor, row);
}

/// Delete a single line at the specified row.
/// Lines below shift up within the scroll region.
/// A blank line is added at the bottom of the scroll region.
fn delete_line_at(processor: &mut AnsiToBufferProcessor, row: usize) {
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ 0, /* Some */ |row| row.as_usize());

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ max_row.convert_to_row_index().as_usize(), /* Some */ |row| row.as_usize());

    // Only operate within scroll region and if cursor is within region.
    if row < scroll_top || row > scroll_bottom {
        return;
    }

    // Shift lines up within the scroll region, from deletion point to bottom.
    for shift_row in row..scroll_bottom {
        processor.ofs_buf.buffer[shift_row] =
            processor.ofs_buf.buffer[shift_row + 1].clone();
    }

    // Clear the bottom line of the scroll region (new blank line).
    clear_line(processor, scroll_bottom);
}

/// Clear a line by filling it with blanks.
fn clear_line(processor: &mut AnsiToBufferProcessor, row: usize) {
    processor.ofs_buf.buffer[row].fill(PixelChar::Spacer);
}
