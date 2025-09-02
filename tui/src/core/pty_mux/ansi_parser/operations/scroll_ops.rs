// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scrolling operations.

use super::{super::{ansi_parser_public_api::AnsiToBufferProcessor,
                    param_utils::ParamsExt},
            cursor_ops};
use crate::PixelChar;

/// Move cursor down one line, scrolling the buffer if at bottom.
/// Implements the ESC D (IND) escape sequence.
/// Respects DECSTBM scroll region margins.
pub fn index_down(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height;
    let current_row = processor.ofs_buf.my_pos.row_index.as_usize();

    // Get scroll region boundaries (1-based to 0-based conversion)
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(max_row.as_usize().saturating_sub(1), |row| row.as_usize());

    // Check if we're at the bottom of the scroll region
    if current_row >= scroll_bottom {
        // At scroll region bottom - scroll buffer content up by one line
        scroll_buffer_up(processor);
    } else {
        // Not at scroll region bottom - just move cursor down
        cursor_ops::cursor_down_by_n(processor, 1);
    }
}

/// Move cursor up one line, scrolling the buffer if at top.
/// Implements the ESC M (RI) escape sequence.
/// Respects DECSTBM scroll region margins.
pub fn reverse_index_up(processor: &mut AnsiToBufferProcessor) {
    let current_row = processor.ofs_buf.my_pos.row_index.as_usize();

    // Get scroll region boundaries (1-based to 0-based conversion)
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(0, |row| row.as_usize());

    // Check if we're at the top of the scroll region
    if current_row <= scroll_top {
        // At scroll region top - scroll buffer content down by one line
        scroll_buffer_down(processor);
    } else {
        // Not at scroll region top - just move cursor up
        cursor_ops::cursor_up_by_n(processor, 1);
    }
}

/// Scroll buffer content up by one line (for ESC D at bottom).
/// The top line is lost, and a new empty line appears at bottom.
/// Respects DECSTBM scroll region margins.
pub fn scroll_buffer_up(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height.as_usize();

    // Get scroll region boundaries (1-based to 0-based conversion)
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(0, |row| row.as_usize());
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(max_row.saturating_sub(1), |row| row.as_usize());

    // Shift lines up within the scroll region only
    // For each row from top to (bottom-1), copy the row below it
    for row in scroll_top..scroll_bottom {
        processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row + 1].clone();
    }

    // Clear the bottom line of the scroll region
    for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
        processor.ofs_buf.buffer[scroll_bottom][col] = PixelChar::Spacer;
    }
}

/// Scroll buffer content down by one line (for ESC M at top).
/// The bottom line is lost, and a new empty line appears at top.
/// Respects DECSTBM scroll region margins.
pub fn scroll_buffer_down(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height.as_usize();

    // Get scroll region boundaries (1-based to 0-based conversion)
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(0, |row| row.as_usize());
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(max_row.saturating_sub(1), |row| row.as_usize());

    // Shift lines down within the scroll region only
    for row in (scroll_top + 1..=scroll_bottom).rev() {
        processor.ofs_buf.buffer[row] = processor.ofs_buf.buffer[row - 1].clone();
    }

    // Clear the new top line of the scroll region
    for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
        processor.ofs_buf.buffer[scroll_top][col] = PixelChar::Spacer;
    }
}

/// Handle SU (Scroll Up) - scroll display up by n lines.
pub fn scroll_up(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    for _ in 0..n {
        scroll_buffer_up(processor);
    }
    tracing::trace!("CSI S (SU): Scrolled up {} lines", n);
}

/// Handle SD (Scroll Down) - scroll display down by n lines.
pub fn scroll_down(processor: &mut AnsiToBufferProcessor, params: &vte::Params) {
    let n = params.extract_nth_non_zero(0);
    for _ in 0..n {
        scroll_buffer_down(processor);
    }
    tracing::trace!("CSI T (SD): Scrolled down {} lines", n);
}
