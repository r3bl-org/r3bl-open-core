// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for [`ANSI`] parser implementation.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code

use crate::{OfsBufVT100, PixelChar, TuiStyle, height, width};
use std::num::NonZeroU16;

/// Creates a test `OfsBufVT100` with 10x10 dimensions.
#[must_use]
pub fn create_test_ofs_buf_10r_by_10c() -> OfsBufVT100 {
    OfsBufVT100::new_empty(height(10) + width(10))
}

/// Creates a test `OfsBufVT100` with 20x20 dimensions for larger test scenarios.
#[must_use]
pub fn create_test_ofs_buf_20r_by_20c() -> OfsBufVT100 {
    OfsBufVT100::new_empty(height(20) + width(20))
}

/// Creates a test buffer with numbered lines for easier test verification.
///
/// # Panics
///
/// Panics if the row index is out of bounds.
#[must_use]
pub fn create_numbered_buffer(rows: usize, cols: usize) -> OfsBufVT100 {
    let mut buf = OfsBufVT100::new_empty(height(rows) + width(cols));
    for r in 0..rows {
        let line_text = format!("Line{r:02}");
        for (c, ch) in line_text.chars().enumerate() {
            if c < cols {
                buf.ofs_buf.get_row_mut(r).unwrap()[c] = PixelChar::PlainText {
                    display_char: ch,
                    style: TuiStyle::default(),
                };
            }
        }
        // Fill remaining columns with spaces.
        for c in line_text.len()..cols {
            buf.ofs_buf.get_row_mut(r).unwrap()[c] = PixelChar::Spacer;
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
        .ofs_buf
        .get_row(row)
        .unwrap()
        .iter()
        .take(expected.len())
        .map(|pixel_char| match pixel_char {
            PixelChar::PlainText { display_char, .. } => *display_char,
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
        .ofs_buf
        .get_row(row)
        .unwrap()
        .iter()
        .all(|pixel_char| matches!(pixel_char, crate::PixelChar::Spacer));

    assert!(
        is_blank,
        "Line {row} should be blank but contains non-space characters"
    );
}

/// Test helper for creating [`NonZeroU16`] values.
///
/// This is a convenience function for tests and doc examples to avoid verbose
/// `NonZeroU16::new().unwrap()` calls when constructing terminal coordinates.
///
/// # Panics
/// Panics if value is 0, which indicates a test bug.
///
/// # Examples
/// ```rust
/// use r3bl_tui::{term_col, term_row};
/// use std::num::NonZeroU16;
///
/// let row = term_row(NonZeroU16::new(5).unwrap());
/// let col = term_col(NonZeroU16::new(10).unwrap());
/// assert_eq!(row.as_u16(), 5);
/// assert_eq!(col.as_u16(), 10);
/// ```
#[must_use]
pub fn nz(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).unwrap_or_else(|| panic!("value must be non-zero: {value}"))
}
