// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Basic integration tests for new CSI operations.

use super::tests_fixtures::*;
use crate::ansi_parser::protocols::csi_codes::CsiSequence;

#[test]
fn test_basic_delete_char_integration() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Write simpler text to test delete char
    let text = "ABCDEF";
    let _write_result = ofs_buf.apply_ansi_bytes(text);

    // Move cursor to position 3 (letter 'D') and delete it
    let move_cursor = CsiSequence::CursorHorizontalAbsolute(4); // 1-based, so 4 = position of 'D'
    let delete_char = CsiSequence::DeleteChar(1);
    let sequence = format!("{move_cursor}{delete_char}");
    let _result = ofs_buf.apply_ansi_bytes(sequence);

    // Should now read "ABCEF " (D deleted, chars shifted left, blank at end)
    assert_line_content(&ofs_buf, 0, "ABCEF ");
}

#[test]
fn test_basic_insert_char_integration() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Write some text and test insert char
    let text = "HELLO";
    let _write_result = ofs_buf.apply_ansi_bytes(text);

    // Move cursor to position 3 and insert a blank character
    let move_cursor = CsiSequence::CursorHorizontalAbsolute(3);
    let insert_char = CsiSequence::InsertChar(1);
    let sequence = format!("{move_cursor}{insert_char}");
    let _result = ofs_buf.apply_ansi_bytes(sequence);

    // Should now read "HE LLO" (blank inserted at position 3)
    let actual: String = ofs_buf.buffer[0]
        .iter()
        .take(6) // "HE LLO" is 6 chars
        .map(|pixel_char| match pixel_char {
            crate::PixelChar::PlainText { display_char, .. } => *display_char,
            crate::PixelChar::Spacer | crate::PixelChar::Void => ' ',
        })
        .collect();
    assert_eq!(actual, "HE LLO", "Expected 'HE LLO', got: '{actual}'");
}

#[test]
fn test_basic_erase_char_integration() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Write some text and test erase char
    let text = "HELLO";
    let _write_result = ofs_buf.apply_ansi_bytes(text);

    // Move cursor to position 3 ('L') and erase it
    let move_cursor = CsiSequence::CursorHorizontalAbsolute(3);
    let erase_char = CsiSequence::EraseChar(1);
    let sequence = format!("{move_cursor}{erase_char}");
    let _result = ofs_buf.apply_ansi_bytes(sequence);

    // Should now read "HE LO" (L erased to blank, no shifting)
    let actual: String = ofs_buf.buffer[0]
        .iter()
        .take(5) // "HE LO" is 5 chars
        .map(|pixel_char| match pixel_char {
            crate::PixelChar::PlainText { display_char, .. } => *display_char,
            crate::PixelChar::Spacer | crate::PixelChar::Void => ' ',
        })
        .collect();
    assert_eq!(actual, "HE LO", "Expected 'HE LO', got: '{actual}'");
}

#[test]
fn test_basic_vpa_integration() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Move to specific position and use VPA
    let move_cursor = CsiSequence::CursorPosition {
        row: crate::ansi_parser::term_units::term_row(5),
        col: crate::ansi_parser::term_units::term_col(7),
    };
    let vpa = CsiSequence::VerticalPositionAbsolute(3);
    let sequence = format!("{move_cursor}{vpa}");
    let _result = ofs_buf.apply_ansi_bytes(sequence);

    // Should be at row 2 (0-based), column 6 (0-based)
    assert_eq!(ofs_buf.my_pos, crate::row(2) + crate::col(6));
}
