// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Basic CSI operation tests using conformance data sequences.
//!
//! This module demonstrates the new testing approach using type-safe sequence
//! builders from the conformance_data module instead of hardcoded format strings.

use super::super::{
    conformance_data::{basic_sequences, cursor_sequences},
    test_fixtures::*,
};
use crate::ansi_parser::protocols::csi_codes::CsiSequence;

#[test]
fn test_basic_delete_char_integration() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Write test text using our sequence builder
    let text_sequence = basic_sequences::insert_text("ABCDEF");
    let _write_result = ofs_buf.apply_ansi_bytes(text_sequence);

    // Move to position 4 (letter 'D') and delete it using sequence builder
    let delete_sequence = basic_sequences::move_and_delete_chars(4, 1);
    let _result = ofs_buf.apply_ansi_bytes(delete_sequence);

    // Should now read "ABCEF " (D deleted, chars shifted left, blank at end)
    assert_line_content(&ofs_buf, 0, "ABCEF ");
}

#[test]
fn test_basic_insert_char_integration() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Write test text using our sequence builder
    let text_sequence = basic_sequences::insert_text("HELLO");
    let _write_result = ofs_buf.apply_ansi_bytes(text_sequence);

    // Move to position 3 and insert a blank character using sequence builder
    let insert_sequence = basic_sequences::move_and_insert_chars(3, 1);
    let _result = ofs_buf.apply_ansi_bytes(insert_sequence);

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

    // Write test text using our sequence builder
    let text_sequence = basic_sequences::insert_text("HELLO");
    let _write_result = ofs_buf.apply_ansi_bytes(text_sequence);

    // Move to position 3 ('L') and erase it using sequence builder
    let erase_sequence = basic_sequences::move_and_erase_chars(3, 1);
    let _result = ofs_buf.apply_ansi_bytes(erase_sequence);

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

    // Move to specific position using our sequence builder
    let move_sequence = cursor_sequences::move_to_position(5, 7);
    let _move_result = ofs_buf.apply_ansi_bytes(move_sequence);

    // Use VPA to move to specific row while maintaining column
    let vpa_sequence = cursor_sequences::move_to_row(3);
    let _result = ofs_buf.apply_ansi_bytes(vpa_sequence);

    // Should be at row 2 (0-based), column 6 (0-based)
    assert_eq!(ofs_buf.cursor_pos, crate::row(2) + crate::col(6));
}
