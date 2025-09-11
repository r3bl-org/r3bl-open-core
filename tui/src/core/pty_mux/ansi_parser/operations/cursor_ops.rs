// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor movement operations.

use std::cmp::{max, min};

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::{AbsolutePosition, CursorPositionRequest,
                                          MovementCount},
                   term_units::TermRow};
use crate::{BoundsCheck, BoundsOverflowStatus::Overflowed, ColIndex, Pos, RowIndex, col,
            row};

/// Move cursor up by n lines.
/// Respects DECSTBM scroll region margins.
///
/// Example - Moving cursor up by 2 lines with scroll region
///
/// ```text
/// Before:        Row: 0-based
/// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
///              │  1  │ Line A                              │
///              │  2  │ Line B                              │
///              │  3  │ Line C                              │
///              │  4  │ Line D  ← cursor (row 4, 0-based)  │ ← Move up 2 lines
///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// After CUU 2:
/// max_height=6 ╮     ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤
///              │  1  │ Line A                              │
///              │  2  │ Line B  ← cursor moved here         │
///              │  3  │ Line C                              │
///              │  4  │ Line D                              │
///              │     ├─────────────────────────────────────┤
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// Result: Cursor moved up 2 lines, stops at scroll region boundaries
/// ```
pub fn cursor_up(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    // Extract movement count (guaranteed >= 1 by VT100 spec).
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    let current_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary: RowIndex = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Move cursor up but don't go above scroll region boundary.
    let potential_new_row = current_row - how_many;
    let new_row = if potential_new_row < scroll_top_boundary {
        scroll_top_boundary
    } else {
        potential_new_row
    };
    performer.ofs_buf.my_pos.row_index = new_row;
}

/// Move cursor down by n lines.
/// Respects DECSTBM scroll region margins.
pub fn cursor_down(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    // Extract movement count (guaranteed >= 1 by VT100 spec).
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    let current_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;
    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region = performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary: RowIndex = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Move cursor down but don't go below scroll region boundary.
    let potential_new_row = current_row + how_many;
    let new_row = if potential_new_row > scroll_bottom_boundary {
        scroll_bottom_boundary
    } else {
        potential_new_row
    };
    performer.ofs_buf.my_pos.row_index = new_row;
}

/// Move cursor forward by n columns.
///
/// Example - Moving cursor forward by 3 columns
///
/// ```text
/// Before:
///                     Column: 0-based
///            ╭────── max_width=10 (1-based) ─────╮
///            ▼                                   ▼
///          ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:  0  │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
///          └───┴───┴─▲─┴───┴───┴───┴───┴───┴───┴───┘
///                    ╰ cursor (col 2, 0-based) - move forward 3
///
/// After CUF 3:
///                     Column: 0-based
///            ╭────── max_width=10 (1-based) ─────╮
///            ▼                                   ▼
///          ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Row:  0  │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
///          └───┴───┴───┴───┴───┴─▲─┴───┴───┴───┴───┘
///                                ╰ cursor moved to (col 5, 0-based)
///
/// Result: Cursor moved forward 3 columns, stops at right margin if would overflow
/// ```
pub fn cursor_forward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_col_width(params);
    let max_col = /* 1-based */ performer.ofs_buf.window_size.col_width;
    let new_col = performer.ofs_buf.my_pos.col_index + how_many;
    // Clamp to max_col-1 if it would overflow.
    performer.ofs_buf.my_pos.col_index = if new_col.check_overflows(max_col) == Overflowed
    {
        max_col.convert_to_col_index()
    } else {
        new_col
    };
}

/// Move cursor backward by n columns.
pub fn cursor_backward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_col_width(params);
    let current_col = /* 0-based */ performer.ofs_buf.my_pos.col_index;
    performer.ofs_buf.my_pos.col_index = current_col - how_many;
}

/// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
/// Respects DECSTBM scroll region margins.
pub fn cursor_position(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let request = CursorPositionRequest::from(params);
    let row_param = /* 1-based ANSI */ request.row;
    let col_param = /* 1-based ANSI */ request.col;

    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;
    let max_col = /* 1-based */ performer.ofs_buf.window_size.col_width;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_top = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary: RowIndex = maybe_scroll_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_bottom = performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary: RowIndex = maybe_scroll_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Clamp row to scroll region bounds and column to buffer bounds.
    let new_row = min(
        max(row_param, scroll_top_boundary.into()),
        scroll_bottom_boundary.into(),
    );
    let new_col = min(col_param, max_col.convert_to_col_index().into());

    performer.ofs_buf.my_pos = Pos {
        col_index: col(new_col),
        row_index: row(new_row),
    };
}

// Internal helper functions for use by other modules (with direct u16 parameters).

/// Internal helper: Move cursor up by n lines (direct parameter).
pub fn cursor_up_by_n(performer: &mut AnsiToOfsBufPerformer, n: RowIndex) {
    let how_many = max(n.as_u16(), 1); // Ensure at least 1 movement
    let current_row = performer.ofs_buf.my_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary: RowIndex = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Move cursor up but don't go above scroll region boundary.
    let new_row = max(
        current_row.as_u16().saturating_sub(how_many),
        scroll_top_boundary.into(),
    );
    performer.ofs_buf.my_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor down by n lines (direct parameter).
pub fn cursor_down_by_n(performer: &mut AnsiToOfsBufPerformer, n: RowIndex) {
    let how_many = max(n.as_u16(), 1); // Ensure at least 1 movement
    let current_row = performer.ofs_buf.my_pos.row_index;
    let max_row = performer.ofs_buf.window_size.row_height;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region = performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary: RowIndex = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Move cursor down but don't go below scroll region boundary.
    let new_row = min(
        current_row.as_u16() + how_many,
        scroll_bottom_boundary.into(),
    );
    performer.ofs_buf.my_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor forward by n columns (direct parameter).
pub fn cursor_forward_by_n(performer: &mut AnsiToOfsBufPerformer, n: ColIndex) {
    let how_many = max(n.as_u16(), 1);
    let max_col = performer.ofs_buf.window_size.col_width;
    let new_col = performer.ofs_buf.my_pos.col_index + how_many;
    // Clamp to max_col-1 if it would overflow.
    performer.ofs_buf.my_pos.col_index = if new_col.check_overflows(max_col) == Overflowed
    {
        max_col.convert_to_col_index()
    } else {
        new_col
    };
}

/// Internal helper: Move cursor backward by n columns (direct parameter).
pub fn cursor_backward_by_n(performer: &mut AnsiToOfsBufPerformer, n: ColIndex) {
    let how_many = max(n.as_u16(), 1);
    let current_col = performer.ofs_buf.my_pos.col_index;
    performer.ofs_buf.my_pos.col_index = current_col - how_many;
}

/// Handle CNL (Cursor Next Line) - move cursor to beginning of line n lines down.
pub fn cursor_next_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    cursor_down_by_n(performer, how_many.convert_to_row_index());
    performer.ofs_buf.my_pos.col_index = col(0);
}

/// Handle CPL (Cursor Previous Line) - move cursor to beginning of line n lines up.
pub fn cursor_prev_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    cursor_up_by_n(performer, how_many.convert_to_row_index());
    performer.ofs_buf.my_pos.col_index = col(0);
}

/// Handle CHA (Cursor Horizontal Absolute) - move cursor to column n (1-based).
pub fn cursor_column(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    // Convert from 1-based to 0-based, clamp to buffer width.
    let target_col = AbsolutePosition::parse_as_col_index(params);
    let max_col_index = performer
        .ofs_buf
        .window_size
        .col_width
        .convert_to_col_index();
    performer.ofs_buf.my_pos.col_index = if target_col > max_col_index {
        max_col_index
    } else {
        target_col
    };
}

/// Handle SCP (Save Cursor Position) - save current cursor position.
pub fn save_cursor_position(performer: &mut AnsiToOfsBufPerformer) {
    performer
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore = Some(performer.ofs_buf.my_pos);
}

/// Handle RCP (Restore Cursor Position) - restore saved cursor position.
pub fn restore_cursor_position(performer: &mut AnsiToOfsBufPerformer) {
    if let Some(saved_pos) = performer
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore
    {
        performer.ofs_buf.my_pos = saved_pos;
    }
}

/// Handle VPA (Vertical Position Absolute) - move cursor to specified row.
/// The horizontal position remains unchanged.
/// Row parameter is 1-based, with default value of 1.
pub fn vertical_position_absolute(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
) {
    let target_row = AbsolutePosition::parse_as_row_index(params);
    let max_row = performer.ofs_buf.window_size.row_height;

    // Clamp to valid range (conversion from 1-based to 0-based already done).
    let new_row = min(target_row, max_row.convert_to_row_index());

    // Update only the row, preserve column.
    performer.ofs_buf.my_pos.row_index = new_row;
}
