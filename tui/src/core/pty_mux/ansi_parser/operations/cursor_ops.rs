// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor movement operations.

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor, param_utils::ParamsExt};
use crate::{BoundsCheck, BoundsStatus::Overflowed, Pos, col, row};

/// Move cursor up by n lines.
/// Respects DECSTBM scroll region margins.
pub fn cursor_up(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_non_zero(0);
    let current_row: u16 = processor.ofs_buf.my_pos.row_index.into();

    // Get scroll region boundaries (1-based to 0-based conversion).
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1 to 0-based
        .map_or(0, std::convert::Into::into);

    // Clamp cursor movement to scroll region top.
    let new_row = u16::max(current_row.saturating_sub(n), scroll_top);
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Move cursor down by n lines.
/// Respects DECSTBM scroll region margins.
pub fn cursor_down(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_non_zero(0);
    let current_row: u16 = processor.ofs_buf.my_pos.row_index.into();
    let max_row: u16 = processor.ofs_buf.window_size.row_height.into();

    // Get scroll region boundaries (1-based to 0-based conversion).
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(max_row.saturating_sub(1), std::convert::Into::into);

    // Clamp cursor movement to scroll region bottom.
    let new_row = u16::min(current_row + n, scroll_bottom);
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Move cursor forward by n columns.
pub fn cursor_forward(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_non_zero(0);
    let max_col = processor.ofs_buf.window_size.col_width;
    let new_col = processor.ofs_buf.my_pos.col_index + col(n);
    // Clamp to max_col-1 if it would overflow.
    processor.ofs_buf.my_pos.col_index = if new_col.check_overflows(max_col) == Overflowed
    {
        max_col.convert_to_col_index()
    } else {
        new_col
    };
}

/// Move cursor backward by n columns.
pub fn cursor_backward(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_non_zero(0);
    #[allow(clippy::cast_possible_truncation)]
    let current_col = processor.ofs_buf.my_pos.col_index.as_usize() as u16;
    processor.ofs_buf.my_pos.col_index = col(current_col.saturating_sub(n));
}

/// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
/// Respects DECSTBM scroll region margins.
pub fn cursor_position(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let row_param = params.extract_nth_non_zero(0).saturating_sub(1);
    let col_param = params.extract_nth_non_zero(1).saturating_sub(1);

    let max_row: u16 = processor.ofs_buf.window_size.row_height.into();
    let max_col: u16 = processor.ofs_buf.window_size.col_width.into();

    // Get scroll region boundaries (1-based to 0-based conversion).
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(0, std::convert::Into::into);
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(max_row.saturating_sub(1), std::convert::Into::into);

    // Clamp row to scroll region bounds and column to buffer bounds.
    let new_row = u16::max(row_param, scroll_top).min(scroll_bottom);
    let new_col = col_param.min(max_col.saturating_sub(1));

    processor.ofs_buf.my_pos = Pos {
        col_index: col(new_col),
        row_index: row(new_row),
    };
}

// Internal helper functions for use by other modules (with direct u16 parameters).

/// Internal helper: Move cursor up by n lines (direct parameter).
pub fn cursor_up_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let n = u16::max(n, 1);
    let current_row: u16 = processor.ofs_buf.my_pos.row_index.into();

    // Get scroll region boundaries (1-based to 0-based conversion).
    let scroll_top = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_top
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(0, std::convert::Into::into);

    // Clamp cursor movement to scroll region top.
    let new_row = u16::max(current_row.saturating_sub(n), scroll_top);
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor down by n lines (direct parameter).
pub fn cursor_down_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let n = u16::max(n, 1);
    let current_row: u16 = processor.ofs_buf.my_pos.row_index.into();
    let max_row: u16 = processor.ofs_buf.window_size.row_height.into();

    // Get scroll region boundaries (1-based to 0-based conversion).
    let scroll_bottom = processor
        .ofs_buf
        .ansi_parser_support
        .scroll_region_bottom
        .and_then(super::super::term_units::TermRow::to_zero_based) // Convert 1-based to 0-based
        .map_or(max_row.saturating_sub(1), std::convert::Into::into);

    // Clamp cursor movement to scroll region bottom.
    let new_row = u16::min(current_row + n, scroll_bottom);
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor forward by n columns (direct parameter).
pub fn cursor_forward_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let n = u16::max(n, 1);
    let max_col = processor.ofs_buf.window_size.col_width;
    let new_col = processor.ofs_buf.my_pos.col_index + col(n);
    // Clamp to max_col-1 if it would overflow.
    processor.ofs_buf.my_pos.col_index = if new_col.check_overflows(max_col) == Overflowed
    {
        max_col.convert_to_col_index()
    } else {
        new_col
    };
}

/// Internal helper: Move cursor backward by n columns (direct parameter).
pub fn cursor_backward_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let n = u16::max(n, 1);
    #[allow(clippy::cast_possible_truncation)]
    let current_col = processor.ofs_buf.my_pos.col_index.as_usize() as u16;
    processor.ofs_buf.my_pos.col_index = col(current_col.saturating_sub(n));
}

/// Handle CNL (Cursor Next Line) - move cursor to beginning of line n lines down.
pub fn cursor_next_line(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_non_zero(0);
    cursor_down_by_n(processor, n);
    processor.ofs_buf.my_pos.col_index = col(0);
    tracing::trace!("CSI E (CNL): Moved to next line {}", n);
}

/// Handle CPL (Cursor Previous Line) - move cursor to beginning of line n lines up.
pub fn cursor_prev_line(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_non_zero(0);
    cursor_up_by_n(processor, n);
    processor.ofs_buf.my_pos.col_index = col(0);
    tracing::trace!("CSI F (CPL): Moved to previous line {}", n);
}

/// Handle CHA (Cursor Horizontal Absolute) - move cursor to column n (1-based).
pub fn cursor_column(
    processor: &mut AnsiToBufferProcessor,
    params: &Params,
) {
    let n = params.extract_nth_non_zero(0);
    // Convert from 1-based to 0-based, clamp to buffer width.
    let target_col = n.saturating_sub(1);
    let max_col: u16 = processor.ofs_buf.window_size.col_width.into();
    processor.ofs_buf.my_pos.col_index = col(target_col.min(max_col.saturating_sub(1)));
    tracing::trace!("CSI G (CHA): Moved to column {}", n);
}

/// Handle SCP (Save Cursor Position) - save current cursor position.
pub fn save_cursor_position(processor: &mut AnsiToBufferProcessor) {
    processor
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore = Some(processor.ofs_buf.my_pos);
    tracing::trace!(
        "CSI s (SCP): Saved cursor position {:?}",
        processor.ofs_buf.my_pos
    );
}

/// Handle RCP (Restore Cursor Position) - restore saved cursor position.
pub fn restore_cursor_position(processor: &mut AnsiToBufferProcessor) {
    if let Some(saved_pos) = processor
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore
    {
        processor.ofs_buf.my_pos = saved_pos;
        tracing::trace!("CSI u (RCP): Restored cursor position to {:?}", saved_pos);
    }
}
