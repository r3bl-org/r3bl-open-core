// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Basic ANSI sequence patterns for fundamental terminal operations.
//!
//! This module provides simple, single-purpose sequence builders that form the
//! foundation for more complex sequence patterns. These functions demonstrate
//! type-safe sequence construction using the codebase's builder types.
//!
//! ## VT100 Specification References
//!
//! - Clear operations: VT100 User Guide Section 3.3.4
//! - Cursor positioning: VT100 User Guide Section 3.3.1
//! - Basic movement: VT100 User Guide Section 3.3.2

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{LengthOps, len, term_col, term_row,
            vt_100_ansi_parser::protocols::csi_codes::CsiSequence};
use std::num::NonZeroU16;

/// Clear entire screen and return cursor to home position (1,1).
///
/// **VT100 Spec**: ESC[2J (Erase Display) + ESC[H (Cursor Home)
///
/// This is one of the most common terminal initialization sequences,
/// used by applications to start with a clean screen state.
///
/// # Example Usage
/// ```rust,ignore
/// let sequence = basic_sequences::clear_and_home();
/// ofs_buf.apply_ansi_bytes(sequence);
/// // Screen is cleared and cursor at top-left
/// ```
#[must_use]
pub fn clear_and_home() -> String {
    format!(
        "{}{}",
        CsiSequence::EraseDisplay(2), // Clear entire screen
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        }  // Move to home position
    )
}

/// Move cursor to specific position and print text.
///
/// **VT100 Spec**: ESC[{row};{col}H (Cursor Position)
///
/// Combines cursor positioning with text output for precise placement
/// of content on the screen.
///
/// # Arguments
/// * `row` - Target row (1-based, VT100 convention)
/// * `col` - Target column (1-based, VT100 convention)
/// * `text` - Text to print at the specified position
#[must_use]
pub fn move_and_print(row: NonZeroU16, col: NonZeroU16, text: &str) -> String {
    format!(
        "{}{}",
        CsiSequence::CursorPosition {
            row: term_row(row),
            col: term_col(col)
        },
        text
    )
}

/// Simple text insertion at cursor position.
///
/// This function provides a consistent way to insert text without
/// any escape sequences, useful for testing basic character handling.
///
/// # Arguments
/// * `text` - Text to insert at current cursor position
#[must_use]
pub fn insert_text(text: &str) -> String { text.to_string() }

/// Move to column and delete characters.
///
/// **VT100 Spec**: ESC[{col}G (Cursor Horizontal Absolute) + ESC[{count}P (Delete Char)
///
/// Common pattern for cursor positioning followed by character deletion.
///
/// # Arguments
/// * `col` - Target column (1-based, VT100 convention)
/// * `count` - Number of characters to delete
#[must_use]
pub fn move_and_delete_chars(col: u16, count: usize) -> String {
    let delete_count = len(count).clamp_to_max(u16::MAX).as_u16();
    format!(
        "{}{}",
        CsiSequence::CursorHorizontalAbsolute(col),
        CsiSequence::DeleteChar(delete_count)
    )
}

/// Move to column and insert blank characters.
///
/// **VT100 Spec**: ESC[{col}G (Cursor Horizontal Absolute) + ESC[{count}@ (Insert Char)
///
/// Common pattern for cursor positioning followed by character insertion.
///
/// # Arguments
/// * `col` - Target column (1-based, VT100 convention)
/// * `count` - Number of blank characters to insert
#[must_use]
pub fn move_and_insert_chars(col: u16, count: usize) -> String {
    let insert_count = len(count).clamp_to_max(u16::MAX).as_u16();
    format!(
        "{}{}",
        CsiSequence::CursorHorizontalAbsolute(col),
        CsiSequence::InsertChar(insert_count)
    )
}

/// Move to column and erase characters.
///
/// **VT100 Spec**: ESC[{col}G (Cursor Horizontal Absolute) + ESC[{count}X (Erase Char)
///
/// Common pattern for cursor positioning followed by character erasure.
///
/// # Arguments
/// * `col` - Target column (1-based, VT100 convention)
/// * `count` - Number of characters to erase (replace with spaces)
#[must_use]
pub fn move_and_erase_chars(col: u16, count: usize) -> String {
    let erase_count = len(count).clamp_to_max(u16::MAX).as_u16();
    format!(
        "{}{}",
        CsiSequence::CursorHorizontalAbsolute(col),
        CsiSequence::EraseChar(erase_count)
    )
}
