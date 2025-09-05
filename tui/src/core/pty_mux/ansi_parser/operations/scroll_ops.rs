// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scrolling operations.

use super::{super::{ansi_parser_public_api::AnsiToBufferProcessor,
                    protocols::csi_codes::MovementCount, term_units::TermRow},
            cursor_ops};
use crate::{PixelChar, row};

/// Move cursor down one line, scrolling the buffer if at bottom.
/// Implements the ESC D (IND) escape sequence.
/// Respects DECSTBM scroll region margins.
pub fn index_down(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height;
    let current_row = processor.ofs_buf.my_pos.row_index;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region = processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Check if we're at the bottom of the scroll region.
    if current_row >= scroll_bottom_boundary {
        // At scroll region bottom - scroll buffer content up by one line.
        scroll_buffer_up(processor);
    } else {
        // Not at scroll region bottom - just move cursor down.
        cursor_ops::cursor_down_by_n(processor, row(1));
    }
}

/// Move cursor up one line, scrolling the buffer if at top.
/// Implements the ESC M (RI) escape sequence.
/// Respects DECSTBM scroll region margins.
pub fn reverse_index_up(processor: &mut AnsiToBufferProcessor) {
    let current_row = processor.ofs_buf.my_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Check if we're at the top of the scroll region.
    if current_row <= scroll_top_boundary {
        // At scroll region top - scroll buffer content down by one line.
        scroll_buffer_down(processor);
    } else {
        // Not at scroll region top - just move cursor up.
        cursor_ops::cursor_up_by_n(processor, row(1));
    }
}

/// Scroll buffer content up by one line (for ESC D at bottom).
/// The top line is lost, and a new empty line appears at bottom.
/// Respects DECSTBM scroll region margins.
pub fn scroll_buffer_up(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(0, |row| row.as_usize());

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(max_row.convert_to_row_index().as_usize(), |row| {
            row.as_usize()
        });

    // Shift lines up within the scroll region only.
    // Note: We can't use copy_within() here because PixelCharLine doesn't implement Copy
    // (it contains heap-allocated data), unlike PixelChar used in char_ops.rs
    for row in scroll_top..scroll_bottom {
        processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row + 1].clone();
    }

    // Clear the bottom line of the scroll region.
    processor.ofs_buf.buffer[scroll_bottom].fill(PixelChar::Spacer);
}

/// Scroll buffer content down by one line (for ESC M at top).
/// The bottom line is lost, and a new empty line appears at top.
/// Respects DECSTBM scroll region margins.
pub fn scroll_buffer_down(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(0, |row| row.as_usize());

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(max_row.convert_to_row_index().as_usize(), |row| {
            row.as_usize()
        });

    // Shift lines down within the scroll region only.
    // Note: We can't use copy_within() here because PixelCharLine doesn't implement Copy
    // (it contains heap-allocated data), unlike PixelChar used in char_ops.rs
    for row in (scroll_top + 1..=scroll_bottom).rev() {
        processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row - 1].clone();
    }

    // Clear the new top line of the scroll region.
    processor.ofs_buf.buffer[scroll_top].fill(PixelChar::Spacer);
}

/// Handle SU (Scroll Up) - scroll display up by n lines.
pub fn scroll_up(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let lines_to_scroll_up = MovementCount::parse_as_row_height(params);
    for _ in 0..lines_to_scroll_up.as_u16() {
        scroll_buffer_up(processor);
    }
}

/// Handle SD (Scroll Down) - scroll display down by n lines.
pub fn scroll_down(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let lines_to_scroll_down = MovementCount::parse_as_row_height(params);
    for _ in 0..lines_to_scroll_down.as_u16() {
        scroll_buffer_down(processor);
    }
}
