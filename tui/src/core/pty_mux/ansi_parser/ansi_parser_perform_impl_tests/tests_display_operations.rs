// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for all display operations - SGR styling, character sets, and printing.

use vte::Perform;

use super::create_test_offscreen_buffer_10r_by_10c;
use crate::{ANSIBasicColor, CharacterSet, SgrCode, TuiColor, TuiStyle,
            ansi_parser::{ansi_parser_public_api::AnsiToBufferProcessor, esc_codes},
            offscreen_buffer::test_fixtures_offscreen_buffer::*,
            tui_style_attrib::{self, Bold},
            tui_style_attribs};

/// Tests for SGR (Select Graphic Rendition) styling operations.
pub mod sgr_styling {
    use super::*;

    #[test]
    fn test_sgr_reset_behavior() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        #[allow(clippy::items_after_statements)]
        const RED: &str = "RED";
        #[allow(clippy::items_after_statements)]
        const NORM: &str = "NORM";

        // SGR reset behavior test:
        //
        // Column:  0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ R │ E │ D │ N │ O │ R │ M │   │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //          └─────────┘ └─────────────┘
        //           bold+red   no styling
        //
        // Sequence: ESC[1m ESC[31m "RED" ESC[0m "NORM"

        // Test SGR reset by sending this sequence to the processor:
        // Set bold+red, write "RED", reset all, write "NORM"
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
            processor.process_bytes(
                /* sequence */
                format!(
                    "{bold}{fg_red}{text1}{reset_all}{text2}",
                    bold = SgrCode::Bold,
                    fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::Red),
                    reset_all = SgrCode::Reset,
                    text1 = RED,
                    text2 = NORM
                ),
            );
        } // processor dropped here

        // Verify "RED" has bold and red color
        for (col, expected_char) in RED.chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buffer| {
                    matches!(
                        (style_from_buffer.attribs.bold, style_from_buffer.color_fg),
                        (
                            Some(tui_style_attrib::Bold),
                            Some(TuiColor::Basic(ANSIBasicColor::Red))
                        )
                    )
                },
                "bold red text",
            );
        }

        // Verify "NORM" has no styling (SGR 0 reset everything)
        assert_plain_text_at(&ofs_buf, 0, RED.len(), NORM);
    }

    #[test]
    fn test_sgr_partial_reset() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // SGR partial reset test (SGR 22 resets bold/dim only):
        //
        // Column:  0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ A │ B │   │   │   │   │   │   │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //          └─┘ └─┘
        //            │   └─ B: italic + dark red (bold reset by SGR 22)
        //            └─ A: bold + italic + dark red
        //
        // Sequence: ESC[1m ESC[3m ESC[31m A ESC[22m B

        // Test partial SGR resets (SGR 22 resets bold/dim only)
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Set bold+italic+red, write "A", reset bold/dim only, write "B"
            let sequence = format!(
                "{bold}{italic}{fg_red}A{reset_bold_dim}B",
                bold = SgrCode::Bold,
                italic = SgrCode::Italic,
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                reset_bold_dim = SgrCode::ResetBoldDim
            );
            processor.process_bytes(&sequence);
        }

        // Verify 'A' has bold, italic, and red
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'A',
            |style_from_buffer| {
                matches!(
                    (
                        style_from_buffer.attribs.bold,
                        style_from_buffer.attribs.italic,
                        style_from_buffer.color_fg
                    ),
                    (
                        Some(tui_style_attrib::Bold),
                        Some(tui_style_attrib::Italic),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "bold italic red",
        );

        // Verify 'B' has italic and red but NOT bold (SGR 22 reset bold/dim)
        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buffer| {
                matches!(
                    (
                        style_from_buffer.attribs.bold,
                        style_from_buffer.attribs.italic,
                        style_from_buffer.color_fg
                    ),
                    (
                        None,
                        Some(tui_style_attrib::Italic),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                )
            },
            "italic red (no bold)",
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
        // Sequence: ESC[30mB ESC[31mR ESC[32mG ESC[37mW ESC[0m  ESC[41mX ESC[42mY ESC[0m ESC[31mESC[44mZ ESC[0m

        // Test various SGR color sequences through VTE parser
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Test foreground colors: black, red, green, white, then background colors
            let sequence = format!(
                "{fg_black}B{fg_red}R{fg_green}G{fg_white}W{reset} {bg_red}X{bg_green}Y{reset2}{fg_red2}{bg_blue}Z{reset3}",
                fg_black = SgrCode::ForegroundBasic(ANSIBasicColor::Black),
                fg_red = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                fg_green = SgrCode::ForegroundBasic(ANSIBasicColor::DarkGreen),
                fg_white = SgrCode::ForegroundBasic(ANSIBasicColor::White),
                reset = SgrCode::Reset,
                bg_red = SgrCode::BackgroundBasic(ANSIBasicColor::DarkRed),
                bg_green = SgrCode::BackgroundBasic(ANSIBasicColor::DarkGreen),
                reset2 = SgrCode::Reset,
                fg_red2 = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
                bg_blue = SgrCode::BackgroundBasic(ANSIBasicColor::DarkBlue),
                reset3 = SgrCode::Reset
            );
            processor.process_bytes(&sequence);
        }

        // Verify colors in buffer
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'B',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::Black))
                )
            },
            "black foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'R',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'G',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                )
            },
            "green foreground",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            3,
            'W',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_fg,
                    Some(TuiColor::Basic(ANSIBasicColor::White))
                )
            },
            "white foreground",
        );

        assert_plain_char_at(&ofs_buf, 0, 4, ' '); // Space after reset

        assert_styled_char_at(
            &ofs_buf,
            0,
            5,
            'X',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                )
            },
            "red background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            6,
            'Y',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::DarkGreen))
                )
            },
            "green background",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            7,
            'Z',
            |style_from_buffer| {
                matches!(
                    (style_from_buffer.color_fg, style_from_buffer.color_bg),
                    (
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed)),
                        Some(TuiColor::Basic(ANSIBasicColor::DarkBlue))
                    )
                )
            },
            "red on blue",
        );
    }
}

/// Tests for character set switching operations.
pub mod character_sets {
    use super::*;

    #[test]
    fn test_esc_character_set_switching() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Start with ASCII mode and write 'q'
            let seq = esc_codes::EscSequence::SelectAscii.to_string();
            processor.process_bytes(&seq); // ESC ( B - Select ASCII
            processor.print('q');

            // Switch to DEC graphics mode
            let seq = esc_codes::EscSequence::SelectGraphics.to_string();
            processor.process_bytes(&seq); // ESC ( 0 - Select DEC graphics

            // Write 'q' which should be translated to '─' (horizontal line)
            processor.print('q');

            // Write 'x' which should be translated to '│' (vertical line)
            processor.print('x');

            // Switch back to ASCII
            let seq = esc_codes::EscSequence::SelectAscii.to_string();
            processor.process_bytes(&seq);

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
}

/// Tests for character printing with styles.
pub mod printing {
    use super::*;

    #[test]
    fn test_print_character_with_styles() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Process styled character
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
            processor.current_style = Some(TuiStyle {
                attribs: tui_style_attribs(Bold),
                color_fg: Some(TuiColor::Basic(ANSIBasicColor::DarkRed)),
                ..Default::default()
            });
            processor.print('S');

            // Verify cursor advanced
            assert_eq!(processor.cursor_pos.col_index.as_usize(), 1);
        }

        // Verify the styled character is in the buffer
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'S',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Bold)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
            },
            "bold red style",
        );
    }
}