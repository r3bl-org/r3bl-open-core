// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test fixtures and helper functions for offscreen buffer testing.
//!
//! This module provides assertion functions that are used by various test modules
//! to verify the state of the offscreen buffer contents.

use crate::{CanvasStorage, Flat2DArray, LengthOps, NarrowingCastToU16, OfsBuf,
            OfsBufVT100, PixelChar, PixelCharLine, SPACER_GLYPH_CHAR, TuiStyle,
            VPHeight, VPPos, VPSize, VPWidth, vp_col, vp_row};

pub trait TestBuffer {
    fn get_char(&self, pos: VPPos) -> Option<PixelChar>;
    fn get_window_size(&self) -> VPSize;
}

impl<S: CanvasStorage> TestBuffer for OfsBuf<S> {
    fn get_char(&self, pos: VPPos) -> Option<PixelChar> { self.get_char(pos) }
    fn get_window_size(&self) -> VPSize { self.get_window_size() }
}

impl TestBuffer for OfsBufVT100 {
    fn get_char(&self, pos: VPPos) -> Option<PixelChar> { self.get_char(pos) }
    fn get_window_size(&self) -> VPSize { self.get_window_size() }
}

/// Assert that a plain character exists at the given position.
/// This function checks that:
/// 1. The position is within buffer bounds
/// 2. The character at that position matches the expected character
/// 3. The character is plain text (not styled)
///
/// # Panics
///
/// Panics if the position is out of bounds or if the character doesn't match.
pub fn assert_plain_char_at(
    buffer: &impl TestBuffer,
    row_idx: usize,
    col_idx: usize,
    expected_char: char,
) {
    let pos = vp_row(row_idx.as_u16_narrowing()) + vp_col(col_idx.as_u16_narrowing());
    let window_size = buffer.get_window_size();

    // Check bounds.
    assert!(
        pos.col_index <= window_size.col_width.convert_to_index(),
        "Column {} is out of bounds (width: {})",
        pos.col_index.as_usize(),
        window_size.col_width.as_usize()
    );
    assert!(
        pos.row_index <= window_size.row_height.convert_to_index(),
        "Row {} is out of bounds (height: {})",
        pos.row_index.as_usize(),
        window_size.row_height.as_usize()
    );

    // Get the character.
    let actual_pixel_char = buffer
        .get_char(pos)
        .unwrap_or_else(|| panic!("No character found at position {pos:?}"));

    // Check it's the expected plain character.
    match actual_pixel_char {
        PixelChar::PlainText {
            display_char,
            style,
        } => {
            assert_eq!(
                style,
                TuiStyle::default(),
                "Expected default style at {pos:?}, but found styled text"
            );
            assert_eq!(
                display_char, expected_char,
                "Expected '{expected_char}' at {pos:?}, but found '{display_char}'",
            );
        }
        other => {
            panic!(
                "Expected plain char '{expected_char}' at {pos:?}, but found {other:?}",
            );
        }
    }
}

/// Assert that a styled character exists at the given position.
/// This function checks that:
/// 1. The position is within buffer bounds
/// 2. The character at that position matches the expected character
/// 3. The character has the expected style (validated by predicate)
///
/// # Panics
///
/// Panics if the position is out of bounds or if the character/style doesn't match.
pub fn assert_styled_char_at<F>(
    buffer: &impl TestBuffer,
    row_idx: usize,
    col_idx: usize,
    expected_char: char,
    style_predicate: F,
    description: &str,
) where
    F: FnOnce(&TuiStyle) -> bool,
{
    let pos = vp_row(row_idx.as_u16_narrowing()) + vp_col(col_idx.as_u16_narrowing());
    let window_size = buffer.get_window_size();

    // Check bounds.
    assert!(
        pos.col_index <= window_size.col_width.convert_to_index(),
        "Column {} is out of bounds (width: {})",
        pos.col_index.as_usize(),
        window_size.col_width.as_usize()
    );
    assert!(
        pos.row_index <= window_size.row_height.convert_to_index(),
        "Row {} is out of bounds (height: {})",
        pos.row_index.as_usize(),
        window_size.row_height.as_usize()
    );

    // Get the character.
    let actual_pixel_char = buffer
        .get_char(pos)
        .unwrap_or_else(|| panic!("No character found at position {pos:?}"));

    // Check it's the expected styled character.
    match actual_pixel_char {
        PixelChar::PlainText {
            display_char,
            style: actual_style,
        } => {
            assert_eq!(
                display_char, expected_char,
                "Expected '{expected_char}' at {pos:?}, but found '{display_char}'",
            );
            assert!(
                style_predicate(&actual_style),
                "Style predicate failed for {description}: expected style matching '{description}' at {pos:?}, but found {actual_style:?}",
            );
        }
        other => {
            panic!(
                "Expected styled char '{expected_char}' matching '{description}' at {pos:?}, but found {other:?}",
            );
        }
    }
}

/// Assert that a position contains an empty character (Spacer).
/// This function checks that:
/// 1. The position is within buffer bounds
/// 2. The position contains either a Spacer or unstyled space character
///
/// # Panics
///
/// Panics if the position is out of bounds or if the character is not empty.
pub fn assert_empty_at(buffer: &impl TestBuffer, row_idx: usize, col_idx: usize) {
    let pos = vp_row(row_idx.as_u16_narrowing()) + vp_col(col_idx.as_u16_narrowing());
    let window_size = buffer.get_window_size();

    // Check bounds.
    assert!(
        pos.col_index <= window_size.col_width.convert_to_index(),
        "Column {} is out of bounds (width: {})",
        pos.col_index.as_usize(),
        window_size.col_width.as_usize()
    );
    assert!(
        pos.row_index <= window_size.row_height.convert_to_index(),
        "Row {} is out of bounds (height: {})",
        pos.row_index.as_usize(),
        window_size.row_height.as_usize()
    );

    // Get the character.
    let actual_pixel_char = buffer
        .get_char(pos)
        .unwrap_or_else(|| panic!("No character found at position {pos:?}"));

    // Check it's empty.
    match actual_pixel_char {
        PixelChar::Spacer => {
            // This is what we expect.
        }
        PixelChar::PlainText {
            display_char: SPACER_GLYPH_CHAR,
            style,
        } => {
            assert_eq!(
                style,
                TuiStyle::default(),
                "Expected default style for space at {pos:?}, but found styled space"
            );
            // This is what we expect - unstyled space.
        }
        other => {
            panic!("Expected empty/spacer at {pos:?}, but found {other:?}");
        }
    }
}

/// Assert that a plain text string exists starting at the given position.
/// This function checks that:
/// 1. Each position is within buffer bounds
/// 2. Each character in the string matches the expected character at the corresponding
///    position
/// 3. All characters are plain text (not styled)
pub fn assert_plain_text_at(
    buffer: &impl TestBuffer,
    start_row: usize,
    start_col: usize,
    expected_text: &str,
) {
    for (index, expected_char) in expected_text.chars().enumerate() {
        assert_plain_char_at(buffer, start_row, start_col + index, expected_char);
    }
}

/// Creates a test buffer with specified dimensions. This provides a common way to create
/// buffers for testing while allowing each test module to specify the size that makes
/// sense for their tests.
#[must_use]
pub fn create_test_buffer_flat_2d(
    buffer_width: VPWidth,
    buffer_height: VPHeight,
) -> OfsBuf {
    OfsBuf::new(Flat2DArray::new_empty(
        buffer_width + buffer_height,
        PixelChar::Spacer,
    ))
}

#[must_use]
pub fn create_vt100_test_buffer_with_size(
    buffer_width: VPWidth,
    buffer_height: VPHeight,
) -> OfsBufVT100 {
    OfsBufVT100::new_empty(buffer_width + buffer_height)
}

/// Creates a plain text [`PixelChar`] with default styling.
///
/// This is the most common character type used in tests and provides a consistent way to
/// create test characters across modules.
///
/// [`PixelChar`]: crate::tui::PixelChar
#[must_use]
pub fn create_plain_test_char(ch: char) -> PixelChar {
    PixelChar::PlainText {
        display_char: ch,
        style: TuiStyle::default(),
    }
}

/// Creates a test line filled with the specified characters. Creates a [`PixelCharLine`]
/// with the given width, filling it with the provided
///
/// characters (repeating if necessary) or padding with Spacers if not enough characters
/// are provided.
///
/// [`PixelCharLine`]: PixelCharLine
#[must_use]
pub fn create_test_line_with_chars(line_width: VPWidth, chars: &[char]) -> PixelCharLine {
    let mut line = vec![PixelChar::Spacer; line_width.as_usize()];
    for (i, &ch) in chars.iter().enumerate().take(line_width.as_usize()) {
        line[i] = create_plain_test_char(ch);
    }
    PixelCharLine { pixel_chars: line }
}
