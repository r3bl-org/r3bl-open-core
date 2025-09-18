// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for all display operations - SGR styling, character sets, and printing.

use vte::Perform;

use super::super::test_fixtures::*;
use crate::{ANSIBasicColor, CharacterSet, SgrCode,
            offscreen_buffer::ofs_buf_test_fixtures::*,
            tui_style_attrib::{self},
            vt100_ansi_parser::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                                esc_codes}};

/// Tests for SGR (Select Graphic Rendition) styling operations.
pub mod sgr_styling {
    use super::*;
    use crate::{col, row};

    #[test]
    fn test_sgr_reset_behavior() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        #[allow(clippy::items_after_statements)]
        const RED: &str = "RED";
        #[allow(clippy::items_after_statements)]
        const NORM: &str = "NORM";

        // SGR reset behavior test:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ R │ E │ D │ N │ O │ R │ M │ ␩ │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //          └─────────┘ └─────────────┘  ╰─ cursor ends here (r:0,c:7)
        //           bold+red   no styling
        //
        // Sequence: ESC[1m ESC[31m "RED" ESC[0m "NORM"

        // Set bold+red, write "RED", reset all, write "NORM".
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        performer.apply_ansi_bytes(format!(
            "{bold}{fg_red}{text1}{reset_all}{text2}",
            bold = SgrCode::Bold,
            fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::Red),
            text1 = RED,
            reset_all = SgrCode::Reset,
            text2 = NORM
        ));

        // Verify "RED" has bold and red color.
        for (col, expected_char) in RED.chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buf| {
                    style_from_buf.color_fg.unwrap() == ANSIBasicColor::Red.into()
                        && style_from_buf.attribs == tui_style_attrib::Bold.into()
                },
                "bold red text",
            );
        }

        // Verify "NORM" has no styling (SGR 0 reset everything)
        assert_plain_text_at(&ofs_buf, 0, RED.len(), NORM);

        // Verify empty cells after "NORM".
        for col_idx in (RED.len() + NORM.len())..10 {
            assert_empty_at(&ofs_buf, 0, col_idx);
        }

        // Verify final cursor position in ofs_buf.my_pos.
        assert_eq!(
            ofs_buf.cursor_pos,
            row(0) + col(RED.len() + NORM.len()),
            "final cursor position after writing RED and NORM should be at row 0, col 7",
        );
    }

    #[test]
    fn test_sgr_partial_reset() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // SGR partial reset test (SGR 22 resets bold/dim only):
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ A │ B │ ␩ │   │   │   │   │   │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //          └─┘ └─┘  ╰─ cursor ends here (r:0,c:2)
        //           │   └─ B: italic + dark red (bold reset by SGR 22)
        //           └───── A: bold + italic + dark red
        //
        // Sequence: ESC[1m ESC[3m ESC[31m A ESC[22m B

        // Test partial SGR resets (SGR 22 resets bold/dim only)
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set bold+italic+red, write "A", reset bold/dim only, write "B"
        performer.apply_ansi_bytes(format!(
            "{bold}{italic}{fg_red}A{reset_bold_dim}B",
            bold = SgrCode::Bold,
            italic = SgrCode::Italic,
            fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
            reset_bold_dim = SgrCode::ResetBoldDim
        ));

        // Verify 'A' has bold, italic, and red.
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buf| {
                style_from_buf.attribs
                    == tui_style_attrib::Bold + tui_style_attrib::Italic
                    && style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkRed.into()
            },
            "bold italic dark-red",
        );

        // Verify 'B' has italic and red but NOT bold (SGR 22 reset bold/dim)
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buf| {
                style_from_buf.attribs == tui_style_attrib::Italic.into()
                    && style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkRed.into()
            },
            "italic dark-red (no bold)",
        );

        // Verify empty cells after "AB".
        for col_idx in 2..10 {
            assert_empty_at(&ofs_buf, 0, col_idx);
        }

        // Verify final cursor position in ofs_buf.my_pos.
        assert_eq!(
            ofs_buf.cursor_pos,
            row(0) + col(2),
            "final cursor position after writing AB should be at row 0, col 2",
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_sgr_color_attributes() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // SGR color attributes test:
        //
        // Column:  0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ B │ R │ G │ W │   │ X │ Y │ Z │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //          └─┘ └─┘ └─┘ └─┘     └─┘ └─┘ └─┘
        //           │   │   │   │       │   │   └─ Z: red fg + blue bg
        //           │   │   │   │       │   └─ Y: green bg
        //           │   │   │   │       └─ X: red bg
        //           │   │   │   └─ W: white fg
        //           │   │   └─ G: green fg
        //           │   └─ R: red fg
        //           └─ B: black fg
        //
        // Sequence: ESC[30mB ESC[31mR ESC[32mG ESC[37mW ESC[0m ESC[41mX ESC[42mY ESC[0m
        //           ESC[31mESC[44mZ ESC[0m

        // Test various SGR color sequences through VTE parser.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test foreground colors: black, red, green, white, then background colors
        performer.apply_ansi_bytes(format!(
                "{fg_blk}B{fg_red}R{fg_grn}G{fg_wht}W{rst} {bg_red}X{bg_grn}Y{rst}{fg_red}{bg_blu}Z{rst}",
                fg_blk = SgrCode::ForegroundBasic(ANSIBasicColor::Black),
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                fg_grn = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
                fg_wht = SgrCode::ForegroundBasic(ANSIBasicColor::White),
                bg_red = SgrCode::BackgroundBasic(ANSIBasicColor::DarkRed),
                bg_grn = SgrCode::BackgroundBasic(ANSIBasicColor::DarkGreen),
                bg_blu = SgrCode::BackgroundBasic(ANSIBasicColor::DarkBlue),
                rst = SgrCode::Reset,
            ));

        // Verify colors in buffer.
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'B',
            |style_from_buf| {
                style_from_buf.color_fg.unwrap() == ANSIBasicColor::Black.into()
            },
            "black foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'R',
            |style_from_buf| {
                style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkRed.into()
            },
            "red foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'G',
            |style_from_buf| {
                style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkGreen.into()
            },
            "green foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            3,
            'W',
            |style_from_buf| {
                style_from_buf.color_fg.unwrap() == ANSIBasicColor::White.into()
            },
            "white foreground",
        );

        assert_plain_char_at(&ofs_buf, 0, 4, ' '); // Space after reset

        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'X',
            |style_from_buf| {
                style_from_buf.color_bg.unwrap() == ANSIBasicColor::DarkRed.into()
            },
            "red background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            6,
            'Y',
            |style_from_buf| {
                style_from_buf.color_bg.unwrap() == ANSIBasicColor::DarkGreen.into()
            },
            "green background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            7,
            'Z',
            |style_from_buf| {
                style_from_buf.color_fg.unwrap() == ANSIBasicColor::DarkRed.into()
                    && style_from_buf.color_bg.unwrap() == ANSIBasicColor::DarkBlue.into()
            },
            "red on blue",
        );
    }

    #[test]
    fn test_sgr_slow_blink() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test ESC[5m (slow blink)
        performer.apply_ansi_bytes(format!(
            "{code}{text}",
            code = SgrCode::SlowBlink,
            text = "BLINK"
        ));

        // Test each character in "BLINK" for the blink attribute.
        for (col, expected_char) in "BLINK".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buf| style_from_buf.attribs == tui_style_attrib::Blink.into(),
                "slow blink",
            );
        }
    }

    #[test]
    fn test_sgr_rapid_blink() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test ESC[6m (rapid blink) - this tests our bug fix!
        performer.apply_ansi_bytes(format!(
            "{code}{text}",
            code = SgrCode::RapidBlink,
            text = "RAPID"
        ));

        // Test each character in "RAPID" for the blink attribute.
        for (col, expected_char) in "RAPID".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buf| style_from_buf.attribs == tui_style_attrib::Blink.into(),
                "rapid blink",
            );
        }
    }

    #[test]
    fn test_sgr_both_blink_types_equivalent() {
        let mut ofs_buf1 = create_test_offscreen_buffer_10r_by_10c();
        let mut ofs_buf2 = create_test_offscreen_buffer_10r_by_10c();

        // Test that both SGR 5 and SGR 6 produce the same result.
        let mut performer1 = AnsiToOfsBufPerformer::new(&mut ofs_buf1);
        let mut performer2 = AnsiToOfsBufPerformer::new(&mut ofs_buf2);

        performer1.apply_ansi_bytes(format!(
            "{code}{text}",
            code = SgrCode::SlowBlink,
            text = "A"
        ));
        performer2.apply_ansi_bytes(format!(
            "{code}{text}",
            code = SgrCode::RapidBlink,
            text = "A"
        ));

        // Both should have blink attribute set.
        assert_styled_char_at(
            &ofs_buf1,
            0,
            0,
            'A',
            |style_from_buf| style_from_buf.attribs == tui_style_attrib::Blink.into(),
            "slow blink should work",
        );

        assert_styled_char_at(
            &ofs_buf2,
            0,
            0,
            'A',
            |style_from_buf| style_from_buf.attribs == tui_style_attrib::Blink.into(),
            "rapid blink should work equivalently",
        );
    }

    #[test]
    fn test_sgr_blink_reset() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Set blink, write char, reset blink, write char.
        performer.apply_ansi_bytes(format!(
            "{c1}{t1}{c2}{t2}",
            c1 = SgrCode::SlowBlink,
            c2 = SgrCode::ResetBlink,
            t1 = "A",
            t2 = "B"
        ));

        // First char should have blink.
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buf| style_from_buf.attribs == tui_style_attrib::Blink.into(),
            "blink enabled",
        );

        // Second char should not have blink (should be plain text)
        assert_plain_char_at(&ofs_buf, 0, 1, 'B');
    }
}

/// Tests for character set switching operations.
pub mod character_sets {
    use super::*;

    #[test]
    fn test_esc_character_set_switching() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Start with ASCII mode and write 'q'.
        performer.apply_ansi_bytes(esc_codes::EscSequence::SelectAscii.to_string());
        performer.print('q');

        // Switch to DEC graphics mode.
        performer.apply_ansi_bytes(esc_codes::EscSequence::SelectDECGraphics.to_string());

        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::DECGraphics
        );

        // Currently in DEC graphics mode.
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Write 'q' which should be translated to '─' (horizontal line)
        performer.print('q');

        // Write 'x' which should be translated to '│' (vertical line)
        performer.print('x');

        // Switch back to ASCII.
        performer.apply_ansi_bytes(esc_codes::EscSequence::SelectAscii.to_string());

        // Write 'q' again (should be normal 'q')
        performer.print('q');

        // Verify character set state after performer is dropped.
        assert_eq!(
            ofs_buf.ansi_parser_support.character_set,
            CharacterSet::Ascii
        );

        // Verify the characters.
        assert_plain_char_at(&ofs_buf, 0, 0, 'q'); // ASCII 'q'
        assert_plain_char_at(&ofs_buf, 0, 1, '─'); // DEC graphics 'q' -> horizontal line
        assert_plain_char_at(&ofs_buf, 0, 2, '│'); // DEC graphics 'x' -> vertical line
        assert_plain_char_at(&ofs_buf, 0, 3, 'q'); // ASCII 'q' again
    }
}
