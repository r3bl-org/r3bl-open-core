// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor control sequence patterns for precise cursor manipulation.
//!
//! This module provides sequences for all cursor movement operations including
//! relative movement, absolute positioning, and save/restore functionality.
//! Both CSI and ESC variants are provided where applicable for compatibility testing.
//!
//! ## VT100 Specification References
//!
//! - Cursor Movement: VT100 User Guide Section 3.3.1
//! - Save/Restore: VT100 User Guide Section 3.3.3
//! - Absolute Positioning: VT100 User Guide Section 3.3.4

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{term_col, term_row,
            vt_100_ansi_parser::protocols::{csi_codes::CsiSequence,
                                            esc_codes::EscSequence}};
use std::num::NonZeroU16;

/// Move cursor to absolute position (row, col).
///
/// **VT100 Spec**: ESC[{row};{col}H or ESC[{row};{col}f (Cursor Position)
///
/// # Arguments
/// * `row` - Target row (1-based, VT100 convention)
/// * `col` - Target column (1-based, VT100 convention)
#[must_use]
pub fn move_to_position(row: NonZeroU16, col: NonZeroU16) -> String {
    CsiSequence::CursorPosition {
        row: term_row(row),
        col: term_col(col),
    }
    .to_string()
}

/// Move cursor to home position (1,1).
///
/// **VT100 Spec**: ESC[H (Cursor Position without parameters)
#[must_use]
pub fn move_to_home() -> String {
    CsiSequence::CursorPosition {
        row: term_row(nz(1)),
        col: term_col(nz(1)),
    }
    .to_string()
}

/// Move cursor up by specified number of lines.
///
/// **VT100 Spec**: ESC[{count}A (Cursor Up)
///
/// # Arguments
/// * `count` - Number of lines to move up (default 1 if count is 0)
#[must_use]
pub fn move_up(count: u16) -> String {
    let move_count = if count == 0 { 1 } else { count };
    CsiSequence::CursorUp(move_count).to_string()
}

/// Move cursor down by specified number of lines.
///
/// **VT100 Spec**: ESC[{count}B (Cursor Down)
///
/// # Arguments
/// * `count` - Number of lines to move down (default 1 if count is 0)
#[must_use]
pub fn move_down(count: u16) -> String {
    let move_count = if count == 0 { 1 } else { count };
    CsiSequence::CursorDown(move_count).to_string()
}

/// Move cursor right by specified number of columns.
///
/// **VT100 Spec**: ESC[{count}C (Cursor Forward)
///
/// # Arguments
/// * `count` - Number of columns to move right (default 1 if count is 0)
#[must_use]
pub fn move_right(count: u16) -> String {
    let move_count = if count == 0 { 1 } else { count };
    CsiSequence::CursorForward(move_count).to_string()
}

/// Move cursor left by specified number of columns.
///
/// **VT100 Spec**: ESC[{count}D (Cursor Backward)
///
/// # Arguments
/// * `count` - Number of columns to move left (default 1 if count is 0)
#[must_use]
pub fn move_left(count: u16) -> String {
    let move_count = if count == 0 { 1 } else { count };
    CsiSequence::CursorBackward(move_count).to_string()
}

/// Save current cursor position and attributes (CSI variant).
///
/// **VT100 Spec**: ESC[s (Save Cursor Position)
///
/// Modern CSI-based save operation. Use with [`restore_cursor_csi()`].
#[must_use]
pub fn save_cursor_csi() -> String { CsiSequence::SaveCursor.to_string() }

/// Restore previously saved cursor position and attributes (CSI variant).
///
/// **VT100 Spec**: ESC[u (Restore Cursor Position)
///
/// Modern CSI-based restore operation. Use with [`save_cursor_csi()`].
#[must_use]
pub fn restore_cursor_csi() -> String { CsiSequence::RestoreCursor.to_string() }

/// Save current cursor position and attributes (ESC variant).
///
/// **VT100 Spec**: ESC 7 (Save Cursor)
///
/// Legacy ESC-based save operation. Use with [`restore_cursor_esc()`].
/// Functionally identical to CSI variant but uses older syntax.
#[must_use]
pub fn save_cursor_esc() -> String { EscSequence::SaveCursor.to_string() }

/// Restore previously saved cursor position and attributes (ESC variant).
///
/// **VT100 Spec**: ESC 8 (Restore Cursor)
///
/// Legacy ESC-based restore operation. Use with [`save_cursor_esc()`].
/// Functionally identical to CSI variant but uses older syntax.
#[must_use]
pub fn restore_cursor_esc() -> String { EscSequence::RestoreCursor.to_string() }

/// Save cursor, perform operation, then restore cursor.
///
/// This is a common pattern in terminal applications where you need to
/// temporarily move the cursor for some operation (like drawing a status line)
/// then return to the original position.
///
/// # Arguments
/// * `operation` - Sequence to execute while cursor is saved
/// * `use_esc` - If true, use ESC 7/8; if false, use CSI s/u
#[must_use]
pub fn save_do_restore(operation: &str, use_esc: bool) -> String {
    if use_esc {
        format!("{}{}{}", save_cursor_esc(), operation, restore_cursor_esc())
    } else {
        format!("{}{}{}", save_cursor_csi(), operation, restore_cursor_csi())
    }
}

/// Move cursor to next line (equivalent to LF + CR).
///
/// **VT100 Spec**: ESC E (Next Line)
///
/// Moves cursor to beginning of next line, combining the effects
/// of Line Feed and Carriage Return.
///
/// Note: Using direct escape sequence since `NextLine` variant doesn't exist yet.
#[must_use]
pub fn next_line() -> String { "\x1bE".to_string() }

/// Move cursor to specific row while maintaining current column.
///
/// **VT100 Spec**: ESC[{row}d (Vertical Position Absolute)
///
/// # Arguments
/// * `row` - Target row (1-based, VT100 convention)
#[must_use]
pub fn move_to_row(row: u16) -> String {
    CsiSequence::VerticalPositionAbsolute(row).to_string()
}

/// Complex cursor movement pattern: draw a box outline.
///
/// Demonstrates combining multiple cursor movements to create
/// a visual pattern useful for testing cursor positioning accuracy.
///
/// # Arguments
/// * `top_row` - Top edge row position
/// * `left_col` - Left edge column position
/// * `width` - Box width in characters
/// * `height` - Box height in characters
#[must_use]
pub fn draw_box_outline(top_row: u16, left_col: u16, width: u16, height: u16) -> String {
    let mut sequence = String::new();

    // Top edge.
    sequence.push_str(&move_to_position(nz(top_row), nz(left_col)));
    sequence.push_str(&"+".repeat(width as usize));

    // Side edges.
    for row in (top_row + 1)..(top_row + height - 1) {
        sequence.push_str(&move_to_position(nz(row), nz(left_col)));
        sequence.push('+');
        sequence.push_str(&move_to_position(nz(row), nz(left_col + width - 1)));
        sequence.push('+');
    }

    // Bottom edge.
    sequence.push_str(&move_to_position(nz(top_row + height - 1), nz(left_col)));
    sequence.push_str(&"+".repeat(width as usize));

    sequence
}
