// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for character insertion, deletion, and erasure operations (ICH/DCH/ECH).
//!
//! Tests the complete pipeline from ANSI sequences through the shim to implementation
//! using the public [`apply_ansi_bytes`] API. This provides integration testing coverage
//! for the [`char_ops`] shim layer. The `test_` prefix follows our naming convention.
//! See [parser module docs] for the complete testing philosophy.
//!
//! **Related Files:**
//! - **Shim**: [`char_ops`] - Parameter translation (tested indirectly by this module)
//! - **Implementation**: [`impl_char_ops`] - Business logic (has separate unit tests)
//!
//! [`apply_ansi_bytes`]: crate::tui::terminal_lib_backends::offscreen_buffer::OffscreenBuffer::apply_ansi_bytes
//! [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::vt_100_shim_char_ops
//! [`impl_char_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_char_ops
//! [parser module docs]: super::super

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{TuiStyle, vt_100_ansi_parser::protocols::csi_codes::CsiSequence};

/// Helper to create a buffer with "ABCDEFGHIJ" in the first row.
fn create_alphabet_buffer() -> crate::OffscreenBuffer {
    let mut buf = create_test_offscreen_buffer_10r_by_10c();
    let alphabet = "ABCDEFGHIJ";
    for (i, ch) in alphabet.chars().enumerate() {
        buf.buffer[0][i] = crate::PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        };
    }
    buf
}

/// Tests for Delete Character (DCH) operations.
pub mod delete_char {
    use super::*;

    #[test]
    fn test_delete_single_char() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 3 (letter 'D') and delete one character
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(4); // Move to column 4 (1-based)
        let delete_char = CsiSequence::DeleteChar(1);
        let sequence = format!("{move_cursor}{delete_char}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "ABC" + "EFGHIJ" + " " (D deleted, chars shifted left, blank at end)
        assert_line_content(&ofs_buf, 0, "ABCEFGHIJ ");
    }

    #[test]
    fn test_delete_multiple_chars() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 2 (letter 'C') and delete three characters
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(3); // Move to column 3 (1-based)
        let delete_chars = CsiSequence::DeleteChar(3);
        let sequence = format!("{move_cursor}{delete_chars}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "AB" + "FGHIJ" + "   " (CDE deleted, chars shifted left, 3 blanks at
        // end).
        assert_line_content(&ofs_buf, 0, "ABFGHIJ   ");
    }

    #[test]
    fn test_delete_chars_at_end_of_line() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 8 (letter 'I') and delete 5 chars
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(9); // Move to column 9 (1-based)
        let delete_chars = CsiSequence::DeleteChar(5);
        let sequence = format!("{move_cursor}{delete_chars}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "ABCDEFGH" + "  " (IJ deleted, 2 blanks at end)
        assert_line_content(&ofs_buf, 0, "ABCDEFGH  ");
    }

    #[test]
    fn test_delete_chars_beyond_right_margin_ignored() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor beyond right margin and try to delete.
        // The cursor will be clamped to column 10, and delete will happen there.
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(11); // Move to column 11 (beyond)
        let delete_char = CsiSequence::DeleteChar(1);
        let sequence = format!("{move_cursor}{delete_char}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // When cursor is clamped to position 10, DeleteChar will delete 'J' at position
        // 10, leaving a space at the end.
        assert_line_content(&ofs_buf, 0, "ABCDEFGHI ");
    }
}

/// Tests for Insert Character (ICH) operations.
pub mod insert_char {
    use super::*;

    #[test]
    fn test_insert_single_char() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 3 (letter 'D') and insert one character.
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(4); // Move to column 4 (1-based)
        let insert_char = CsiSequence::InsertChar(1);
        let sequence = format!("{move_cursor}{insert_char}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "ABC" + " " + "DEFGHI" (blank inserted, chars shifted right, J lost).
        assert_line_content(&ofs_buf, 0, "ABC DEFGHI");
    }

    #[test]
    fn test_insert_multiple_chars() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 2 (letter 'C') and insert three characters
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(3); // Move to column 3 (1-based)
        let insert_chars = CsiSequence::InsertChar(3);
        let sequence = format!("{move_cursor}{insert_chars}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "AB" + "   " + "CDEFG" (3 blanks inserted, chars shifted right, HIJ
        // lost).
        assert_line_content(&ofs_buf, 0, "AB   CDEFG");
    }

    #[test]
    fn test_insert_chars_at_end_of_line() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 8 (letter 'I') and insert three characters
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(9); // Move to column 9 (1-based)
        let insert_chars = CsiSequence::InsertChar(3);
        let sequence = format!("{move_cursor}{insert_chars}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "ABCDEFGH" + "  " (only 2 blanks can be inserted, IJ lost)
        assert_line_content(&ofs_buf, 0, "ABCDEFGH  ");
    }

    #[test]
    fn test_insert_chars_beyond_right_margin_ignored() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor beyond right margin and try to insert.
        // The cursor will be clamped to column 10, and insert will happen there.
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(11); // Move to column 11 (beyond)
        let insert_char = CsiSequence::InsertChar(1);
        let sequence = format!("{move_cursor}{insert_char}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // When cursor is clamped to position 10, InsertChar will insert a space at
        // position 10, pushing 'J' off the right edge.
        assert_line_content(&ofs_buf, 0, "ABCDEFGHI ");
    }
}

/// Tests for Erase Character (ECH) operations.
pub mod erase_char {
    use super::*;

    #[test]
    fn test_erase_single_char() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 3 (letter 'D') and erase one character
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(4); // Move to column 4 (1-based)
        let erase_char = CsiSequence::EraseChar(1);
        let sequence = format!("{move_cursor}{erase_char}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "ABC" + " " + "EFGHIJ" (D erased to blank, no shifting)
        assert_line_content(&ofs_buf, 0, "ABC EFGHIJ");
    }

    #[test]
    fn test_erase_multiple_chars() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 2 (letter 'C') and erase three characters
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(3); // Move to column 3 (1-based)
        let erase_chars = CsiSequence::EraseChar(3);
        let sequence = format!("{move_cursor}{erase_chars}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "AB" + "   " + "FGHIJ" (CDE erased to blanks, no shifting)
        assert_line_content(&ofs_buf, 0, "AB   FGHIJ");
    }

    #[test]
    fn test_erase_chars_at_end_of_line() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor to column 8 (letter 'I') and erase five characters
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(9); // Move to column 9 (1-based)
        let erase_chars = CsiSequence::EraseChar(5);
        let sequence = format!("{move_cursor}{erase_chars}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // Verify: "ABCDEFGH" + "  " (IJ erased to blanks)
        assert_line_content(&ofs_buf, 0, "ABCDEFGH  ");
    }

    #[test]
    fn test_erase_chars_beyond_right_margin_ignored() {
        let mut ofs_buf = create_alphabet_buffer();

        // Move cursor beyond right margin and try to erase.
        // The cursor will be clamped to column 10, and erase will happen there.
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(11); // Move to column 11 (beyond)
        let erase_char = CsiSequence::EraseChar(1);
        let sequence = format!("{move_cursor}{erase_char}");
        let _result = ofs_buf.apply_ansi_bytes(sequence);

        // When cursor is clamped to position 10, EraseChar will erase 'J' at position 10,
        // replacing it with a space.
        assert_line_content(&ofs_buf, 0, "ABCDEFGHI ");
    }

    #[test]
    fn test_erase_vs_delete_difference() {
        let mut ofs_buf_erase = create_alphabet_buffer();
        let mut ofs_buf_delete = create_alphabet_buffer();

        // Both operations at column 3 (letter 'D')
        let move_cursor = CsiSequence::CursorHorizontalAbsolute(4); // Move to column 4 (1-based)

        // Erase one character: ESC[X
        let erase_sequence = format!("{move_cursor}{}", CsiSequence::EraseChar(1));
        let _result1 = ofs_buf_erase.apply_ansi_bytes(erase_sequence);

        // Delete one character: ESC[P
        let delete_sequence = format!("{move_cursor}{}", CsiSequence::DeleteChar(1));
        let _result2 = ofs_buf_delete.apply_ansi_bytes(delete_sequence);

        // Verify different results:
        // Erase: D becomes blank, no shifting
        assert_line_content(&ofs_buf_erase, 0, "ABC EFGHIJ");

        // Delete: D removed, chars shift left, blank at end
        assert_line_content(&ofs_buf_delete, 0, "ABCEFGHIJ ");
    }
}
