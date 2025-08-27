// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for ESC (Escape) sequence operations.

use crate::{ansi_parser::{csi_codes::CsiSequence, esc_codes},
            CharacterSet, Pos, col, row,
            offscreen_buffer::test_fixtures_offscreen_buffer::*};
use super::tests_parse_common::create_test_offscreen_buffer_10r_by_10c;
use crate::ansi_parser::ansi_parser_perform_impl::{new, process_bytes};
use vte::Perform;

#[test]
fn test_csi_save_restore_cursor() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Move to position and save with CSI s
        processor.cursor_pos = row(3) + col(5);
        processor.print('A');

        // CSI s - Save cursor position
        let sequence = CsiSequence::SaveCursor.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        // Move elsewhere
        processor.cursor_pos = row(7) + col(2);
        processor.print('B');

        // CSI u - Restore cursor position
        let sequence = CsiSequence::RestoreCursor.to_string();
        process_bytes(&mut processor, &mut parser, sequence);

        // Should be back at saved position
        assert_eq!(processor.cursor_pos.row_index.as_usize(), 3);
        assert_eq!(processor.cursor_pos.col_index.as_usize(), 6); // After 'A'

        processor.print('C');
    }

    // Verify characters
    assert_plain_char_at(&ofs_buf, 3, 5, 'A');
    assert_plain_char_at(&ofs_buf, 7, 2, 'B');
    assert_plain_char_at(&ofs_buf, 3, 6, 'C'); // At restored position
}

#[test]
fn test_esc_save_restore_cursor() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Move cursor to position (3, 5) and write 'A'
        processor.cursor_pos = row(3) + col(5);
        processor.print('A');

        // Save cursor position at (3, 6) using ESC 7
        let seq = esc_codes::EscSequence::SaveCursor.to_string();
        process_bytes(&mut processor, &mut parser, &seq);

        // Move cursor elsewhere and write 'B'
        processor.cursor_pos = row(7) + col(2);
        processor.print('B');

        // Restore cursor position using ESC 8
        let seq = esc_codes::EscSequence::RestoreCursor.to_string();
        process_bytes(&mut processor, &mut parser, &seq);

        // Verify cursor was restored
        assert_eq!(
            processor.cursor_pos,
            Pos {
                row_index: row(3),
                col_index: col(6),
            }
        );

        // Write 'C' at restored position
        processor.print('C');
    }

    // Verify saved cursor position persisted in buffer
    assert_eq!(
        ofs_buf.ansi_parser_support.saved_cursor_pos,
        Some(row(3) + col(6))
    );

    // Verify characters are at expected positions
    assert_plain_char_at(&ofs_buf, 3, 5, 'A');
    assert_plain_char_at(&ofs_buf, 7, 2, 'B');
    assert_plain_char_at(&ofs_buf, 3, 6, 'C'); // Should be right after 'A'
}

#[test]
fn test_esc_character_set_switching() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Start with ASCII mode and write 'q'
        let seq = esc_codes::EscSequence::SelectAscii.to_string();
        process_bytes(&mut processor, &mut parser, &seq); // ESC ( B - Select ASCII
        processor.print('q');

        // Switch to DEC graphics mode
        let seq = esc_codes::EscSequence::SelectGraphics.to_string();
        process_bytes(&mut processor, &mut parser, &seq); // ESC ( 0 - Select DEC graphics

        // Write 'q' which should be translated to '─' (horizontal line)
        processor.print('q');

        // Write 'x' which should be translated to '│' (vertical line)
        processor.print('x');

        // Switch back to ASCII
        let seq = esc_codes::EscSequence::SelectAscii.to_string();
        process_bytes(&mut processor, &mut parser, &seq);

        // Write 'q' again (should be normal 'q')
        processor.print('q');
    }

    // Verify character set state after processor is dropped
    assert_eq!(ofs_buf.ansi_parser_support.character_set, CharacterSet::Ascii);

    // Verify the characters
    assert_plain_char_at(&ofs_buf, 0, 0, 'q'); // ASCII 'q'
    assert_plain_char_at(&ofs_buf, 0, 1, '─'); // DEC graphics 'q' -> horizontal line
    assert_plain_char_at(&ofs_buf, 0, 2, '│'); // DEC graphics 'x' -> vertical line
    assert_plain_char_at(&ofs_buf, 0, 3, 'q'); // ASCII 'q' again
}

// Add more ESC sequence tests as needed...