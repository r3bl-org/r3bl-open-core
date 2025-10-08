// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for ANSI parser implementation.

use crate::{OffscreenBuffer, TuiStyle, height, width};
use std::num::NonZeroU16;

/// Create a test `OffscreenBuffer` with 10x10 dimensions.
#[must_use]
pub fn create_test_offscreen_buffer_10r_by_10c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(10) + width(10))
}

/// Create a test `OffscreenBuffer` with 20x20 dimensions for larger test scenarios.
#[must_use]
pub fn create_test_offscreen_buffer_20r_by_20c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(20) + width(20))
}

/// Create a test buffer with numbered lines for easier test verification.
#[must_use]
pub fn create_numbered_buffer(rows: usize, cols: usize) -> OffscreenBuffer {
    let mut buf = OffscreenBuffer::new_empty(height(rows) + width(cols));
    for r in 0..rows {
        let line_text = format!("Line{r:02}");
        for (c, ch) in line_text.chars().enumerate() {
            if c < cols {
                buf.buffer[r][c] = crate::PixelChar::PlainText {
                    display_char: ch,
                    style: TuiStyle::default(),
                };
            }
        }
        // Fill remaining columns with spaces.
        for c in line_text.len()..cols {
            buf.buffer[r][c] = crate::PixelChar::Spacer;
        }
    }
    buf
}

/// Helper to verify line content matches expected text.
///
/// # Panics
/// Panics if `row` is out of bounds for the buffer.
pub fn assert_line_content(buf: &OffscreenBuffer, row: usize, expected: &str) {
    let actual: String = buf.buffer[row]
        .iter()
        .take(expected.len())
        .map(|pixel_char| match pixel_char {
            crate::PixelChar::PlainText { display_char, .. } => *display_char,
            crate::PixelChar::Spacer | crate::PixelChar::Void => ' ',
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
pub fn assert_blank_line(buf: &OffscreenBuffer, row: usize) {
    let is_blank = buf.buffer[row]
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
/// use r3bl_tui::vt_100_ansi_parser::term_units::{term_row, term_col};
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
