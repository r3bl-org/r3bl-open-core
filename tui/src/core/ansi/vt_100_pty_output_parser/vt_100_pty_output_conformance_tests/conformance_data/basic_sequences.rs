// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Basic ANSI sequence patterns for fundamental terminal operations.
//!
//! This module provides simple, single-purpose sequence builders that form the foundation
//! for more complex sequence patterns. These functions demonstrate type-safe sequence
//! construction using the codebase's builder types.
//!
//! ## [`VT-100`] Specification References
//!
//! - Clear operations: [VT-100 specification Section 3.3.4]
//! - Cursor positioning: [VT-100 specification Section 3.3.1]
//! - Basic movement: [VT-100 specification Section 3.3.2]
//!
//! ## Functions Overview
//!
//! - [`clear_and_home`] - Clear entire screen and return cursor to home position (1,1)
//! - [`move_and_print`] - Move cursor to specific position and print text
//! - [`insert_text`] - Simple text insertion at cursor position
//! - [`move_and_delete_chars`] - Move to column and delete characters
//! - [`move_and_insert_chars`] - Move to column and insert blank characters
//! - [`move_and_erase_chars`] - Move to column and erase characters
//!
//! [VT-100 specification Section 3.3.1]:
//!     https://vt100.net/docs/vt100-ug/chapter3.html#S3.3.1
//! [VT-100 specification Section 3.3.2]:
//!     https://vt100.net/docs/vt100-ug/chapter3.html#S3.3.2
//! [VT-100 specification Section 3.3.4]:
//!     https://vt100.net/docs/vt100-ug/chapter3.html#S3.3.4
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-100` specification]: https://vt100.net/docs/vt100-ug/

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{CsiCount, EraseDisplayMode, TermCol,
            core::ansi::vt_100_pty_output_parser::CsiSequence, term_col, term_row};
use std::num::NonZeroU16;

/// Clear entire screen and return cursor to home position (1,1).
///
/// **[`VT-100`] Spec**: `ESC [ 2 J` (Erase Display) + `ESC [ H` (Cursor Home)
///
/// This is one of the most common terminal initialization sequences, used by applications
/// to start with a clean screen state.
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[must_use]
pub fn clear_and_home() -> String {
    format!(
        "{}{}",
        // Clear entire screen
        CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen),
        // Move to home position
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        }
    )
}

/// Move cursor to specific position and print text.
///
/// **[`VT-100`] Spec**: `ESC [ {row} ; {col} H` (Cursor Position)
///
/// Combines cursor positioning with text output for precise placement of content on the
/// screen.
///
/// # Arguments
/// * `row` - Target row (1-based, [`VT-100`] convention)
/// * `col` - Target column (1-based, [`VT-100`] convention)
/// * `text` - Text to print at the specified position
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
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
/// This function provides a consistent way to insert text without any escape sequences,
/// useful for testing basic character handling.
///
/// # Arguments
/// * `text` - Text to insert at current cursor position
#[must_use]
pub fn insert_text(text: &str) -> String { text.to_string() }

/// Move to column and delete characters.
///
/// **[`VT-100`] Spec**: `ESC [ {col} G` (Cursor Horizontal Absolute) + `ESC [ {count} P`
/// (Delete Char)
///
/// Common pattern for cursor positioning followed by character deletion.
///
/// # Arguments
/// * `col` - Target column.
/// * `count` - Number of characters to delete.
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[must_use]
pub fn move_and_delete_chars(col: TermCol, count: CsiCount) -> String {
    format!(
        "{}{}",
        CsiSequence::CursorHorizontalAbsolute(col),
        CsiSequence::DeleteChar(count)
    )
}

/// Move to column and insert blank characters.
///
/// **[`VT-100`] Spec**: `ESC [ {col} G` (Cursor Horizontal Absolute) + `ESC [ {count} @`
/// (Insert Char)
///
/// Common pattern for cursor positioning followed by character insertion.
///
/// # Arguments
/// * `col` - Target column.
/// * `count` - Number of blank characters to insert.
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[must_use]
pub fn move_and_insert_chars(col: TermCol, count: CsiCount) -> String {
    format!(
        "{}{}",
        CsiSequence::CursorHorizontalAbsolute(col),
        CsiSequence::InsertChar(count)
    )
}

/// Move to column and erase characters.
///
/// **[`VT-100`] Spec**: `ESC [ {col} G` (Cursor Horizontal Absolute) + `ESC [ {count} X`
/// (Erase Char)
///
/// Common pattern for cursor positioning followed by character erasure.
///
/// # Arguments
/// * `col` - Target column.
/// * `count` - Number of characters to erase (replace with spaces).
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[must_use]
pub fn move_and_erase_chars(col: TermCol, count: CsiCount) -> String {
    format!(
        "{}{}",
        CsiSequence::CursorHorizontalAbsolute(col),
        CsiSequence::EraseChar(count)
    )
}
