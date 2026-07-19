// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

//! Test modules for [`ANSI`] parser implementation.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code

use crate::{NarrowingCastToU16, OfsBufVT100, PixelChar, TuiStyle, vp_col, vp_height,
            vp_row, vp_width};
use std::num::NonZeroU16;

/// Creates a test `OfsBufVT100` with 10x10 dimensions.
#[must_use]
pub fn create_test_ofs_buf_10r_by_10c() -> OfsBufVT100 {
    OfsBufVT100::new_empty(vp_height(10) + vp_width(10))
}

/// Creates a test `OfsBufVT100` with 20x20 dimensions for larger test scenarios.
#[must_use]
pub fn create_test_ofs_buf_20r_by_20c() -> OfsBufVT100 {
    OfsBufVT100::new_empty(vp_height(20) + vp_width(20))
}

/// Creates a test buffer with numbered lines for easier test verification.
///
/// # Panics
///
/// Panics if the row index is out of bounds.
#[must_use]
pub fn create_numbered_buffer(rows: usize, cols: usize) -> OfsBufVT100 {
    let mut buf = OfsBufVT100::new_empty(
        vp_height(rows.as_u16_narrowing()) + vp_width(cols.as_u16_narrowing()),
    );
    for row_idx in 0..rows {
        let line_text = format!("Line{row_idx:02}");
        for (col_idx, ch) in line_text.chars().enumerate() {
            if col_idx < cols {
                let _unused = buf.set_char(
                    vp_row(row_idx.as_u16_narrowing())
                        + vp_col(col_idx.as_u16_narrowing()),
                    PixelChar::PlainText {
                        display_char: ch,
                        style: TuiStyle::default(),
                    },
                );
            }
        }
        // Fill remaining columns with spaces.
        for col_idx in line_text.len()..cols {
            let _unused = buf.set_char(
                vp_row(row_idx.as_u16_narrowing()) + vp_col(col_idx.as_u16_narrowing()),
                PixelChar::Spacer,
            );
        }
    }
    buf
}

/// Helper to verify line content matches expected text.
///
/// # Panics
/// Panics if `row` is out of bounds for the buffer.
pub fn assert_line_content(buf: &OfsBufVT100, row: usize, expected: &str) {
    let actual: String = buf
        .get_row((row.as_u16_narrowing()).into())
        .expect("conversion error")
        .iter()
        .take(expected.len())
        .map(|&pixel_char| match pixel_char {
            PixelChar::PlainText { display_char, .. } => display_char,
            PixelChar::Spacer | PixelChar::Void => ' ',
        })
        .collect();

    assert_eq!(
        actual, expected,
        "Line {row} content mismatch. Expected: '{expected}', got: '{actual}'"
    );
}

/// Helper to verify a line contains only blank/space characters.
///
/// # Panics
/// Panics if `row` is out of bounds for the buffer.
pub fn assert_blank_line(buf: &OfsBufVT100, row: usize) {
    let is_blank = buf
        .get_row((row.as_u16_narrowing()).into())
        .expect("conversion error")
        .iter()
        .all(|pixel_char| matches!(pixel_char, PixelChar::Spacer));

    assert!(
        is_blank,
        "Line {row} should be blank but contains non-space characters"
    );
}

/// Test helper for creating [`NonZeroU16`] values.
///
/// This is a convenience function for tests and doc examples to avoid verbose
/// `NonZeroU16::new().expect("conversion error")` calls when constructing terminal
/// coordinates.
///
/// # Panics
/// Panics if value is 0, which indicates a test bug.
///
/// # Examples
/// ```rust
/// use r3bl_tui::{term_col, term_row};
/// use std::num::NonZeroU16;
///
/// let row = term_row(NonZeroU16::new(5).expect("conversion error"));
/// let col = term_col(NonZeroU16::new(10).expect("conversion error"));
/// assert_eq!(row.as_u16(), 5);
/// assert_eq!(col.as_u16(), 10);
/// ```
#[must_use]
pub fn nz(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).unwrap_or_else(|| panic!("value must be non-zero: {value}"))
}
