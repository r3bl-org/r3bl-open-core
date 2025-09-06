// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line insertion and deletion operations.

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   protocols::csi_codes::MovementCount, term_units::TermRow};
use crate::{PixelChar, RowIndex, row};

/// Handle IL (Insert Line) - insert n blank lines at cursor position.
/// Lines below cursor and within scroll region shift down.
/// Lines scrolled off the bottom are lost.
pub fn insert_lines(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let insert_lines_count = MovementCount::parse_as_row_height(params);
    let current_row = processor.ofs_buf.my_pos.row_index;

    for _ in 0..insert_lines_count.as_u16() {
        insert_line_at(processor, current_row);
    }
}

/// Handle DL (Delete Line) - delete n lines starting at cursor position.
/// Lines below cursor and within scroll region shift up.
/// Blank lines are added at the bottom of the scroll region.
pub fn delete_lines(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let delete_lines_count = MovementCount::parse_as_row_height(params);
    let current_row = processor.ofs_buf.my_pos.row_index;

    for _ in 0..delete_lines_count.as_u16() {
        delete_line_at(processor, current_row);
    }
}

/// Insert a single blank line at the specified row.
/// Lines below shift down within the scroll region.
/// The bottom line of the scroll region is lost.
fn insert_line_at(processor: &mut AnsiToBufferProcessor, row_index: RowIndex) {
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Only operate within scroll region and if cursor is within region.
    if row_index < scroll_top || row_index > scroll_bottom {
        return;
    }

    // Shift lines down within the scroll region, from bottom to insertion point.
    // Note: We can't use copy_within() here because PixelCharLine doesn't implement Copy
    // (it contains heap-allocated data), unlike PixelChar used in char_ops.rs
    let row_start = row_index.as_usize();
    let row_end = scroll_bottom.as_usize();

    for shift_row in (row_start + 1..=row_end).rev() {
        if shift_row > row_start {
            processor.ofs_buf.buffer[shift_row] =
                processor.ofs_buf.buffer[shift_row - 1].clone();
        }
    }

    // Clear the newly inserted line.
    clear_line(processor, row_index);
}

/// Delete a single line at the specified row.
/// Lines below shift up within the scroll region.
/// A blank line is added at the bottom of the scroll region.
fn delete_line_at(processor: &mut AnsiToBufferProcessor, row_index: RowIndex) {
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Only operate within scroll region and if cursor is within region.
    if row_index < scroll_top || row_index > scroll_bottom {
        return;
    }

    // Shift lines up within the scroll region, from deletion point to bottom.
    // Note: We can't use copy_within() here because PixelCharLine doesn't implement Copy
    // (it contains heap-allocated data), unlike PixelChar used in char_ops.rs
    let row_start = row_index.as_usize();
    let row_end = scroll_bottom.as_usize();

    for shift_row in row_start..row_end {
        processor.ofs_buf.buffer[shift_row] =
            processor.ofs_buf.buffer[shift_row + 1].clone();
    }

    // Clear the bottom line of the scroll region (new blank line).
    clear_line(processor, scroll_bottom);
}

/// Clear a line by filling it with blanks.
fn clear_line(processor: &mut AnsiToBufferProcessor, row_index: RowIndex) {
    processor.ofs_buf.buffer[row_index.as_usize()].fill(PixelChar::Spacer);
}
