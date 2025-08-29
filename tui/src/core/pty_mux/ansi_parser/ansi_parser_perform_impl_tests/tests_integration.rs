// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for complex ANSI sequences and VTE parser integration.

use super::tests_fixtures::*;
use crate::{ANSIBasicColor, OffscreenBuffer, SgrCode, TuiColor, height, width,
            ansi_parser::{ansi_parser_public_api::AnsiToBufferProcessor,
                          csi_codes::{csi_seq_cursor_pos, CsiSequence},
                          esc_codes::EscSequence,
                          term_units::{term_row, term_col}},
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            tui_style_attrib};

/// Create a test `OffscreenBuffer` with 24x80 dimensions (more realistic terminal size).
fn create_offscreen_buffer_24r_by_80c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(24) + width(80))
}

/// Tests for complex real-world ANSI sequences.
pub mod full_sequences {
    use super::*;

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_vim_like_sequence() {
        let mut ofs_buf = create_offscreen_buffer_24r_by_80c();

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Simulate a vim-like sequence using proper builders
        let sequence = format!(
            "{clear_screen}{home_position}{reverse_video}{status_text}{reset_attrs}{save_cursor}{move_to_cmd}{prompt}{restore_cursor}{restored_text}",
            clear_screen = CsiSequence::EraseDisplay(2),  // ESC[2J
            home_position = csi_seq_cursor_pos(term_row(1) + term_col(1)), // ESC[H
            reverse_video = SgrCode::Invert, // ESC[7m
            status_text = "-- INSERT --",
            reset_attrs = SgrCode::Reset, // ESC[0m
            save_cursor = EscSequence::SaveCursor, // ESC 7
            move_to_cmd = csi_seq_cursor_pos(term_row(24) + term_col(1)), // ESC[24;1H
            prompt = ":",
            restore_cursor = EscSequence::RestoreCursor, // ESC 8
            restored_text = "Hello World!"
        );

        processor.process_bytes(sequence);

        // Verify the sequence worked correctly
        // Status line should be at top with reverse video
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            '-',
            |style| matches!(style.attribs.reverse, Some(tui_style_attrib::Reverse)),
            "reverse video status"
        );

        // Command prompt should be at bottom
        assert_plain_char_at(&ofs_buf, 23, 0, ':');

        // Content should be restored at saved position
        assert_plain_text_at(&ofs_buf, 0, 12, "Hello World!");
    }

    #[test]
    fn test_complex_ansi_sequences() {
        let mut ofs_buf = create_offscreen_buffer_24r_by_80c();

        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Simulate: Bold text, colored text, cursor movement
        let sequence = format!(
            "{bold}Bold{reset1} {fg_green}Green{reset2}",
            bold = SgrCode::Bold,
            reset1 = SgrCode::Reset,
            fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
            reset2 = SgrCode::Reset
        );
        processor.process_bytes(sequence);

        // Verify "Bold" with bold style
        for (i, ch) in "Bold".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                i,
                ch,
                |style_from_buffer| {
                    matches!(style_from_buffer.attribs.bold, Some(tui_style_attrib::Bold))
                },
                "bold style",
            );
        }

        // Verify space at position 4
        assert_plain_char_at(&ofs_buf, 0, 4, ' ');

        // Verify "Green" with green color
        for (i, ch) in "Green".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                5 + i,
                ch,
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                    )
                },
                "green foreground",
            );
        }
    }
}

/// Tests for VTE parser integration.
pub mod vte_parser {
    use super::*;

    #[test]
    fn test_vte_parser_integration() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Process ANSI sequences through VTE parser
        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Print "Hello" with red foreground
        let input = format!(
            "Hello{fg_red}R{reset}",
            fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
            reset = SgrCode::Reset
        );
        processor.process_bytes(&input);

        // Verify cursor position after processing
        assert_eq!(processor.ofs_buf.my_pos.col_index.as_usize(), 6);

        // Verify "Hello" is in the buffer
        assert_plain_text_at(&ofs_buf, 0, 0, "Hello");

        // Verify 'R' has red color
        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'R',
            |style_from_buffer| {
                style_from_buffer.color_fg == Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
            },
            "red foreground",
        );

        // Verify rest of line is empty
        for col in 6..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }
    }
}