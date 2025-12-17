// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for complex ANSI sequences and VTE parser integration.

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{
    ANSIBasicColor, EraseDisplayMode, EscSequence, OffscreenBuffer, SgrCode, height, width,
            offscreen_buffer::test_fixtures_ofs_buf::*,
            term_col, term_row, tui_style_attrib};
use crate::core::ansi::vt_100_pty_output_parser::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                                            CsiSequence,
                                            vt_100_pty_output_conformance_tests::test_sequence_generators::csi_builders::csi_seq_cursor_pos};

/// Create a test `OffscreenBuffer` with 24x80 dimensions (more realistic terminal size).
fn create_offscreen_buffer_24r_by_80c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(24) + width(80))
}

/// Tests for complex real-world ANSI sequences.
mod full_sequences {
    use super::*;

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_vim_like_sequence() {
        let mut ofs_buf = create_offscreen_buffer_24r_by_80c();

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Simulate a vim-like sequence using proper builders.
        let sequence = format!(
            "{clear_screen}{home_position}{reverse_video}{status_text}{reset_attrs}{save_cursor}{move_to_cmd}{prompt}{restore_cursor}{restored_text}",
            clear_screen = CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen), /* ESC[2J */
            home_position = csi_seq_cursor_pos(term_row(nz(1)) + term_col(nz(1))), /* ESC[H */
            reverse_video = SgrCode::Invert, // ESC[7m
            status_text = "-- INSERT --",
            reset_attrs = SgrCode::Reset,          // ESC[0m
            save_cursor = EscSequence::SaveCursor, // ESC 7
            move_to_cmd = csi_seq_cursor_pos(term_row(nz(24)) + term_col(nz(1))), /* ESC[24;1H */
            prompt = ":",
            restore_cursor = EscSequence::RestoreCursor, // ESC 8
            restored_text = "Hello World!"
        );

        performer.apply_ansi_bytes(sequence);

        // Verify the sequence worked correctly.
        // Status line should be at top with reverse video.
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            '-',
            |style_from_buf| style_from_buf.attribs == tui_style_attrib::Reverse.into(),
            "reverse video status",
        );

        // Command prompt should be at bottom.
        assert_plain_char_at(&ofs_buf, 23, 0, ':');

        // Content should be restored at saved position.
        assert_plain_text_at(&ofs_buf, 0, 12, "Hello World!");
    }

    #[test]
    fn test_complex_ansi_sequences() {
        let mut ofs_buf = create_offscreen_buffer_24r_by_80c();

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Simulate: Bold text, colored text, cursor movement
        let sequence = format!(
            "{bold}Bold{reset1} {fg_green}Green{reset2}",
            bold = SgrCode::Bold,
            reset1 = SgrCode::Reset,
            fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
            reset2 = SgrCode::Reset
        );
        performer.apply_ansi_bytes(sequence);

        // Verify "Bold" with bold style.
        for (i, ch) in "Bold".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                i,
                ch,
                |style_from_buf| style_from_buf.attribs == tui_style_attrib::Bold.into(),
                "bold style",
            );
        }

        // Verify space at position 4.
        assert_plain_char_at(&ofs_buf, 0, 4, ' ');

        // Verify "Green" with green color.
        for (i, ch) in "Green".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                5 + i,
                ch,
                |style_from_buf| {
                    style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkGreen.into()
                },
                "green foreground",
            );
        }
    }
}

/// Tests for VTE parser integration.
mod vte_parser {
    use super::*;

    #[test]
    fn test_vte_parser_integration() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Process ANSI sequences through VTE parser.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Print "Hello" with red foreground.
        let input = format!(
            "Hello{fg_red}R{reset}",
            fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
            reset = SgrCode::Reset
        );
        performer.apply_ansi_bytes(&input);

        // Verify cursor position after processing.
        assert_eq!(performer.ofs_buf.cursor_pos.col_index.as_usize(), 6);

        // Verify "Hello" is in the buffer.
        assert_plain_text_at(&ofs_buf, 0, 0, "Hello");

        // Verify 'R' has red color.
        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'R',
            |style_from_buf| {
                style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkRed.into()
            },
            "red foreground",
        );

        // Verify rest of line is empty.
        for col in 6..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }
    }
}
