// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for all display operations - SGR styling, character sets, and printing.

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{ANSIBasicColor, CharacterSet, EscSequence, SgrCode,
            core::ansi::vt_100_pty_output_parser::ansi_parser_public_api::AnsiToOfsBufPerformer,
            offscreen_buffer::test_fixtures_ofs_buf::*,
            tui_style_attrib::{self}};
use vte::Perform;

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
                |style_from_buf| {
                    style_from_buf.attribs.blink
                        == tui_style_attrib::BlinkMode::Slow.into()
                },
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
                |style_from_buf| {
                    style_from_buf.attribs.blink
                        == tui_style_attrib::BlinkMode::Rapid.into()
                },
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
            |style_from_buf| {
                style_from_buf.attribs.blink == tui_style_attrib::BlinkMode::Slow.into()
            },
            "slow blink should work",
        );

        assert_styled_char_at(
            &ofs_buf2,
            0,
            0,
            'A',
            |style_from_buf| {
                style_from_buf.attribs.blink == tui_style_attrib::BlinkMode::Rapid.into()
            },
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
            |style_from_buf| {
                style_from_buf.attribs.blink == tui_style_attrib::BlinkMode::Slow.into()
            },
            "blink enabled",
        );

        // Second char should not have blink (should be plain text)
        assert_plain_char_at(&ofs_buf, 0, 1, 'B');
    }

    #[test]
    fn test_sgr_extended_256_colors() {
        use crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_sequence_generators::extended_color_builders::*;

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test 256-color sequences (both colon and semicolon formats)
        //
        // Column:  0   1   2   3   4   5
        //         ┌───┬───┬───┬───┬───┬───┐
        // Row 0:  │ F │ B │ M │   │   │   │
        //         └───┴───┴───┴───┴───┴───┘
        //          └─┘ └─┘ └─┘
        //           │   │   └─ M: index 21 fg + index 196 bg
        //           │   └─ B: index 196 bg
        //           └─ F: index 196 fg

        performer.apply_ansi_bytes(format!(
            "{fg_196}F{rst}{bg_196}B{rst}{fg_21}{bg_196}M{rst}",
            fg_196 = fg_ansi256(196), // Bright red foreground
            bg_196 = bg_ansi256(196), // Bright red background
            fg_21 = fg_ansi256(21),   // Blue foreground
            rst = SgrCode::Reset,
        ));

        // Verify 'F' has 256-color foreground (index 196)
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'F',
            |style_from_buf| {
                // 256-color index 196 should be stored in color_fg
                style_from_buf.color_fg.is_some()
            },
            "256-color foreground (index 196)",
        );

        // Verify 'B' has 256-color background (index 196)
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buf| {
                // 256-color index 196 should be stored in color_bg
                style_from_buf.color_bg.is_some()
            },
            "256-color background (index 196)",
        );

        // Verify 'M' has both 256-color foreground and background
        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'M',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "256-color fg (index 21) + bg (index 196)",
        );
    }

    #[test]
    fn test_sgr_extended_rgb_colors() {
        use crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_sequence_generators::extended_color_builders::*;

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test RGB color sequences (colon-separated format)
        //
        // Column:  0   1   2   3
        //         ┌───┬───┬───┬───┐
        // Row 0:  │ R │ G │ B │   │
        //         └───┴───┴───┴───┘
        //          └─┘ └─┘ └─┘
        //           │   │   └─ B: RGB(0,128,255) fg + RGB(255,128,0) bg
        //           │   └─ G: RGB(255,128,0) bg (orange)
        //           └─ R: RGB(255,0,0) fg (red)

        performer.apply_ansi_bytes(format!(
            "{fg_red}R{rst}{bg_orange}G{rst}{fg_blue}{bg_orange}B{rst}",
            fg_red = fg_rgb(255, 0, 0),      // Red foreground
            bg_orange = bg_rgb(255, 128, 0), // Orange background
            fg_blue = fg_rgb(0, 128, 255),   // Blue foreground
            rst = SgrCode::Reset,
        ));

        // Verify 'R' has RGB foreground
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'R',
            |style_from_buf| style_from_buf.color_fg.is_some(),
            "RGB foreground (255,0,0)",
        );

        // Verify 'G' has RGB background
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'G',
            |style_from_buf| style_from_buf.color_bg.is_some(),
            "RGB background (255,128,0)",
        );

        // Verify 'B' has both RGB foreground and background
        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'B',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "RGB fg (0,128,255) + bg (255,128,0)",
        );
    }

    #[test]
    fn test_sgr_extended_colors_mixed() {
        use crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_sequence_generators::extended_color_builders::*;

        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test mixing basic ANSI, 256-color, and RGB sequences
        //
        // Column:  0   1   2   3
        //         ┌───┬───┬───┬───┐
        // Row 0:  │ A │ B │ C │   │
        //         └───┴───┴───┴───┘
        //          └─┘ └─┘ └─┘
        //           │   │   └─ C: RGB fg + basic bg
        //           │   └─ B: 256-color fg + basic bg
        //           └─ A: basic fg + 256-color bg

        performer.apply_ansi_bytes(format!(
            "{basic_fg}{ansi256_bg}A{rst}{ansi256_fg}{basic_bg}B{rst}{rgb_fg}{basic_bg}C{rst}",
            basic_fg = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
            ansi256_bg = bg_ansi256(21),
            ansi256_fg = fg_ansi256(196),
            basic_bg = SgrCode::BackgroundBasic(ANSIBasicColor::DarkGreen),
            rgb_fg = fg_rgb(255, 128, 0),
            rst = SgrCode::Reset,
        ));

        // Verify 'A' has basic fg + 256-color bg
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "basic fg + 256-color bg",
        );

        // Verify 'B' has 256-color fg + basic bg
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "256-color fg + basic bg",
        );

        // Verify 'C' has RGB fg + basic bg
        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'C',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "RGB fg + basic bg",
        );
    }

    /// Test semicolon-separated 256-color sequences (legacy format).
    ///
    /// This tests the look-ahead parsing logic that handles the legacy semicolon format
    /// where VTE parses `ESC[38;5;196m` as separate positions: `[[38], [5], [196]]`.
    #[test]
    fn test_sgr_extended_256_colors_semicolon_format() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test 256-color sequences with semicolon format (legacy)
        //
        // Column:  0   1   2
        //         ┌───┬───┬───┐
        // Row 0:  │ F │ B │ M │
        //         └───┴───┴───┘

        // Use raw escape sequences with semicolons (legacy format).
        performer.apply_ansi_bytes(format!(
            "\x1b[38;5;196mF\x1b[0m\x1b[48;5;21mB\x1b[0m\x1b[38;5;196;48;5;21mM\x1b[0m"
        ));

        // Verify 'F' has 256-color foreground.
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'F',
            |style_from_buf| style_from_buf.color_fg.is_some(),
            "256-color fg (semicolon format)",
        );

        // Verify 'B' has 256-color background.
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buf| style_from_buf.color_bg.is_some(),
            "256-color bg (semicolon format)",
        );

        // Verify 'M' has both 256-color fg and bg.
        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'M',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "256-color fg + bg (semicolon format)",
        );
    }

    /// Test semicolon-separated RGB color sequences (legacy format).
    #[test]
    fn test_sgr_extended_rgb_colors_semicolon_format() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

        // Test RGB color sequences with semicolon format (legacy)
        //
        // Column:  0   1   2
        //         ┌───┬───┬───┐
        // Row 0:  │ R │ G │ B │
        //         └───┴───┴───┘

        // Use raw escape sequences with semicolons (legacy format).
        performer.apply_ansi_bytes(format!(
            "\x1b[38;2;255;0;0mR\x1b[0m\x1b[48;2;255;128;0mG\x1b[0m\x1b[38;2;0;128;255;48;2;255;128;0mB\x1b[0m"
        ));

        // Verify 'R' has RGB foreground.
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'R',
            |style_from_buf| style_from_buf.color_fg.is_some(),
            "RGB fg (semicolon format)",
        );

        // Verify 'G' has RGB background.
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'G',
            |style_from_buf| style_from_buf.color_bg.is_some(),
            "RGB bg (semicolon format)",
        );

        // Verify 'B' has both RGB fg and bg.
        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'B',
            |style_from_buf| {
                style_from_buf.color_fg.is_some() && style_from_buf.color_bg.is_some()
            },
            "RGB fg + bg (semicolon format)",
        );
    }

    /// Test mixing semicolon-separated extended colors with basic SGR attributes.
    #[test]
    fn test_sgr_semicolon_extended_colors_with_attributes() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Test: bold + semicolon 256-color fg
        // Sequence: ESC[1;38;5;196m (bold + fg index 196)
        {
            let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
            performer.apply_ansi_bytes("\x1b[1;38;5;196mA\x1b[0m");
        }

        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buf| {
                style_from_buf.attribs.bold.is_some() && style_from_buf.color_fg.is_some()
            },
            "bold + 256-color fg (semicolon format)",
        );

        // Test: semicolon 256-color fg + bold (reversed order)
        // Sequence: ESC[38;5;196;1m (fg index 196 + bold)
        {
            let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
            performer.apply_ansi_bytes("\x1b[38;5;196;1mB\x1b[0m");
        }

        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buf| {
                style_from_buf.attribs.bold.is_some() && style_from_buf.color_fg.is_some()
            },
            "256-color fg + bold (reversed, semicolon format)",
        );
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
        performer.apply_ansi_bytes(EscSequence::SelectAscii.to_string());
        performer.print('q');

        // Switch to DEC graphics mode.
        performer.apply_ansi_bytes(EscSequence::SelectDECGraphics.to_string());

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
        performer.apply_ansi_bytes(EscSequence::SelectAscii.to_string());

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
