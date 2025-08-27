// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for full ANSI sequences that simulate real applications.

use crate::{ANSIBasicColor, SgrCode, TuiColor, height, width, OffscreenBuffer,
            ansi_parser::{csi_codes::CsiSequence, esc_codes::EscSequence},
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            tui_style_attrib};
use crate::ansi_parser::ansi_parser_perform_impl::{new, process_bytes};

/// Create a test `OffscreenBuffer` with 24x80 dimensions (more realistic terminal
/// size).
fn create_offscreen_buffer_24r_by_80c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(24) + width(80))
}

#[test]
#[allow(clippy::items_after_statements)]
fn test_vim_like_sequence() {
    let mut ofs_buf = create_offscreen_buffer_24r_by_80c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Simulate a vim-like sequence using proper builders
        let sequence = format!(
            "{clear_screen}{home_position}{reverse_video}{status_text}{reset_attrs}{save_cursor}{move_to_cmd}{prompt}{restore_cursor}{restored_text}",
            clear_screen = CsiSequence::EraseDisplay(2),  // ESC[2J
            home_position = CsiSequence::CursorPosition { row: 1, col: 1 }, // ESC[H
            reverse_video = SgrCode::Invert, // ESC[7m
            status_text = "-- INSERT --",
            reset_attrs = SgrCode::Reset, // ESC[0m
            save_cursor = EscSequence::SaveCursor, // ESC 7
            move_to_cmd = CsiSequence::CursorPosition { row: 24, col: 1 }, // ESC[24;1H
            prompt = ":",
            restore_cursor = EscSequence::RestoreCursor, // ESC 8
            restored_text = "Hello World!"
        );

        process_bytes(&mut processor, &mut parser, sequence);
    }

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
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Simulate: Bold text, colored text, cursor movement
        let sequence = format!(
            "{bold}Bold{reset1} {fg_green}Green{reset2}",
            bold = SgrCode::Bold,
            reset1 = SgrCode::Reset,
            fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
            reset2 = SgrCode::Reset
        );
        process_bytes(&mut processor, &mut parser, sequence);
    }

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