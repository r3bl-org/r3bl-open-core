// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor movement operations.
//!
//! # CSI Sequence Architecture
//!
//! ```text
//! Application sends "ESC[2A" (cursor up 2 lines)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [THIS METHOD]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use std::cmp::{max, min};

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::{AbsolutePosition, CursorPositionRequest,
                                          MovementCount}};
use crate::{ColIndex, RowIndex, col, row};

/// Move cursor up by n lines.
/// Respects DECSTBM scroll region margins.
/// See `OffscreenBuffer::cursor_up` for detailed behavior and examples.
pub fn cursor_up(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    // Extract movement count (guaranteed >= 1 by VT100 spec).
    let how_many = MovementCount::parse_as_row_height(params);
    performer.ofs_buf.cursor_up(how_many);
}

/// Move cursor down by n lines.
/// Respects DECSTBM scroll region margins.
/// See `OffscreenBuffer::cursor_down` for detailed behavior and examples.
pub fn cursor_down(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    // Extract movement count (guaranteed >= 1 by VT100 spec).
    let how_many = MovementCount::parse_as_row_height(params);
    performer.ofs_buf.cursor_down(how_many);
}

/// Move cursor forward by n columns.
/// See `OffscreenBuffer::cursor_forward` for detailed behavior and examples.
pub fn cursor_forward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_as_col_width(params);
    performer.ofs_buf.cursor_forward(how_many);
}

/// Move cursor backward by n columns.
/// See `OffscreenBuffer::cursor_backward` for detailed behavior and examples.
pub fn cursor_backward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_as_col_width(params);
    performer.ofs_buf.cursor_backward(how_many);
}

/// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
/// Respects DECSTBM scroll region margins.
pub fn cursor_position(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let request = CursorPositionRequest::from(params);
    performer.ofs_buf.cursor_to_position(
        row(/* 1-based ANSI */ request.row),
        col(/* 1-based ANSI */ request.col),
    );
}

// Internal helper functions for use by other modules (with direct u16 parameters).

/// Internal helper: Move cursor up by n lines (direct parameter).
pub fn cursor_up_by_n(performer: &mut AnsiToOfsBufPerformer, n: RowIndex) {
    let how_many = max(n.as_u16(), 1); // Ensure at least 1 movement
    let current_row = performer.ofs_buf.cursor_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let scroll_top_boundary = performer.ofs_buf.get_scroll_top_boundary();

    // Move cursor up but don't go above scroll region boundary.
    let new_row = max(
        current_row.as_u16().saturating_sub(how_many),
        scroll_top_boundary.into(),
    );
    performer.ofs_buf.cursor_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor down by n lines (direct parameter).
pub fn cursor_down_by_n(performer: &mut AnsiToOfsBufPerformer, n: RowIndex) {
    let how_many = max(n.as_u16(), 1); // Ensure at least 1 movement
    let current_row = performer.ofs_buf.cursor_pos.row_index;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let scroll_bottom_boundary = performer.ofs_buf.get_scroll_bottom_boundary();

    // Move cursor down but don't go below scroll region boundary.
    let new_row = min(
        current_row.as_u16() + how_many,
        scroll_bottom_boundary.into(),
    );
    performer.ofs_buf.cursor_pos.row_index = row(new_row);
}

/// Internal helper: Move cursor forward by n columns (direct parameter).
pub fn cursor_forward_by_n(performer: &mut AnsiToOfsBufPerformer, n: ColIndex) {
    let how_many = max(n.as_u16(), 1);
    let new_col = performer.ofs_buf.cursor_pos.col_index + how_many;
    // Clamp to max_col-1 if it would overflow.
    performer.ofs_buf.cursor_pos.col_index = performer.ofs_buf.clamp_column(new_col);
}

/// Internal helper: Move cursor backward by n columns (direct parameter).
pub fn cursor_backward_by_n(performer: &mut AnsiToOfsBufPerformer, n: ColIndex) {
    let how_many = max(n.as_u16(), 1);
    let current_col = performer.ofs_buf.cursor_pos.col_index;
    performer.ofs_buf.cursor_pos.col_index = current_col - how_many;
}

/// Handle CNL (Cursor Next Line) - move cursor to beginning of line n lines down.
pub fn cursor_next_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_as_row_height(params);
    performer.ofs_buf.cursor_down(how_many);
    performer.ofs_buf.cursor_to_line_start();
}

/// Handle CPL (Cursor Previous Line) - move cursor to beginning of line n lines up.
pub fn cursor_prev_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_as_row_height(params);
    performer.ofs_buf.cursor_up(how_many);
    performer.ofs_buf.cursor_to_line_start();
}

/// Handle CHA (Cursor Horizontal Absolute) - move cursor to column n (1-based).
pub fn cursor_column(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let target_col = AbsolutePosition::parse_as_col_index(params);
    performer.ofs_buf.cursor_to_column(target_col);
}

/// Handle SCP (Save Cursor Position) - save current cursor position.
pub fn save_cursor_position(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.save_cursor_position();
}

/// Handle RCP (Restore Cursor Position) - restore saved cursor position.
pub fn restore_cursor_position(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.restore_cursor_position();
}

/// Handle VPA (Vertical Position Absolute) - move cursor to specified row.
/// The horizontal position remains unchanged.
/// Row parameter is 1-based, with default value of 1.
pub fn vertical_position_absolute(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
) {
    let target_row = AbsolutePosition::parse_as_row_index(params);
    performer.ofs_buf.cursor_to_row(target_row);
}
