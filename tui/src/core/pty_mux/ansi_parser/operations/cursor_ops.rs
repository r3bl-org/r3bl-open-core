// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor movement operations.

use std::cmp::{max, min};

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   protocols::csi_codes::{CursorPositionRequest, MovementCount},
                   term_units::TermRow};
use crate::{BoundsCheck, BoundsStatus::Overflowed, Pos, RowIndex, col, row};

/// Move cursor up by n lines.
/// Respects DECSTBM scroll region margins.
pub fn cursor_up(processor: &mut AnsiToBufferProcessor, params: &Params) {
    // Extract movement count (guaranteed >= 1 by VT100 spec).
    let lines_to_move_up = MovementCount::from(params).as_u16();
    let current_row = processor.ofs_buf.my_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary: u16 = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ 0, /* Some */ Into::into);

    // Move cursor up but don't go above scroll region boundary.
    let new_row = max(
        current_row.as_u16().saturating_sub(lines_to_move_up),
        scroll_top_boundary,
    );
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Move cursor down by n lines.
/// Respects DECSTBM scroll region margins.
pub fn cursor_down(processor: &mut AnsiToBufferProcessor, params: &Params) {
    // Extract movement count (guaranteed >= 1 by VT100 spec).
    let lines_to_move_down = MovementCount::from(params).as_u16();
    let current_row = processor.ofs_buf.my_pos.row_index;
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region = processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary: RowIndex = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Move cursor down but don't go below scroll region boundary.
    let new_row = min(
        current_row.as_u16() + lines_to_move_down,
        scroll_bottom_boundary.into(),
    );
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Move cursor forward by n columns.
pub fn cursor_forward(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let cols_to_move_forward = MovementCount::from(params).as_u16();
    let max_col = processor.ofs_buf.window_size.col_width;
    let new_col = processor.ofs_buf.my_pos.col_index + col(cols_to_move_forward);
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
    let cols_to_move_backward = MovementCount::from(params).as_u16();
    let current_col = processor.ofs_buf.my_pos.col_index;
    processor.ofs_buf.my_pos.col_index =
        col(current_col.as_u16().saturating_sub(cols_to_move_backward));
}

/// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
/// Respects DECSTBM scroll region margins.
pub fn cursor_position(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let request = CursorPositionRequest::from(params);
    let row_param = request.row;
    let col_param = request.col;

    let max_row = processor.ofs_buf.window_size.row_height;
    let max_col = processor.ofs_buf.window_size.col_width;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_top = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary: u16 = maybe_scroll_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ 0, /* Some */ Into::into);

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_bottom = processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary: RowIndex = maybe_scroll_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Clamp row to scroll region bounds and column to buffer bounds.
    let new_row = min(
        max(row_param, scroll_top_boundary),
        scroll_bottom_boundary.into(),
    );
    let new_col = min(col_param, max_col.convert_to_col_index().into());

    processor.ofs_buf.my_pos = Pos {
        col_index: col(new_col),
        row_index: row(new_row),
    };
}

// Internal helper functions for use by other modules (with direct u16 parameters).

/// Internal helper: Move cursor up by n lines (direct parameter).
pub fn cursor_up_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let lines_to_move_up = max(n, 1); // Ensure at least 1 movement
    let current_row = processor.ofs_buf.my_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region = processor.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary: u16 = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ 0, /* Some */ Into::into);

    // Move cursor up but don't go above scroll region boundary.
    let new_row = max(
        current_row.as_u16().saturating_sub(lines_to_move_up),
        scroll_top_boundary,
    );
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor down by n lines (direct parameter).
pub fn cursor_down_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let lines_to_move_down = max(n, 1); // Ensure at least 1 movement
    let current_row = processor.ofs_buf.my_pos.row_index;
    let max_row = processor.ofs_buf.window_size.row_height;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region = processor.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary: RowIndex = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Move cursor down but don't go below scroll region boundary.
    let new_row = min(
        current_row.as_u16() + lines_to_move_down,
        scroll_bottom_boundary.into(),
    );
    processor.ofs_buf.my_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor forward by n columns (direct parameter).
pub fn cursor_forward_by_n(processor: &mut AnsiToBufferProcessor, n: u16) {
    let cols_to_move_forward = max(n, 1);
    let max_col = processor.ofs_buf.window_size.col_width;
    let new_col = processor.ofs_buf.my_pos.col_index + col(cols_to_move_forward);
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
    let cols_to_move_backward = max(n, 1);
    let current_col = processor.ofs_buf.my_pos.col_index;
    processor.ofs_buf.my_pos.col_index =
        col(current_col.as_u16().saturating_sub(cols_to_move_backward));
}

/// Handle CNL (Cursor Next Line) - move cursor to beginning of line n lines down.
pub fn cursor_next_line(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let lines_to_move_down = MovementCount::from(params).as_u16();
    cursor_down_by_n(processor, lines_to_move_down);
    processor.ofs_buf.my_pos.col_index = col(0);
}

/// Handle CPL (Cursor Previous Line) - move cursor to beginning of line n lines up.
pub fn cursor_prev_line(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let lines_to_move_up = MovementCount::from(params).as_u16();
    cursor_up_by_n(processor, lines_to_move_up);
    processor.ofs_buf.my_pos.col_index = col(0);
}

/// Handle CHA (Cursor Horizontal Absolute) - move cursor to column n (1-based).
pub fn cursor_column(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let target_column = MovementCount::from(params).as_u16();
    // Convert from 1-based to 0-based, clamp to buffer width.
    let target_col = target_column.saturating_sub(1);
    let max_col = processor.ofs_buf.window_size.col_width;
    processor.ofs_buf.my_pos.col_index =
        col(min(target_col, max_col.convert_to_col_index().into()));
}

/// Handle SCP (Save Cursor Position) - save current cursor position.
pub fn save_cursor_position(processor: &mut AnsiToBufferProcessor) {
    processor
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore = Some(processor.ofs_buf.my_pos);
}

/// Handle RCP (Restore Cursor Position) - restore saved cursor position.
pub fn restore_cursor_position(processor: &mut AnsiToBufferProcessor) {
    if let Some(saved_pos) = processor
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore
    {
        processor.ofs_buf.my_pos = saved_pos;
    }
}

/// Handle VPA (Vertical Position Absolute) - move cursor to specified row.
/// The horizontal position remains unchanged.
/// Row parameter is 1-based, with default value of 1.
pub fn vertical_position_absolute(
    processor: &mut AnsiToBufferProcessor,
    params: &Params,
) {
    let target_row = MovementCount::from(params).as_u16();
    let max_row = processor.ofs_buf.window_size.row_height;

    // Convert from 1-based to 0-based and clamp to valid range.
    let new_row = min(
        target_row.saturating_sub(1),
        max_row.convert_to_row_index().into(),
    );

    // Update only the row, preserve column.
    processor.ofs_buf.my_pos.row_index = row(new_row);
}
