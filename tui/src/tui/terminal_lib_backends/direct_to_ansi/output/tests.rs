// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for [`AnsiSequenceGenerator`]
//!
//! Tests all static methods to verify correct ANSI escape sequence generation.

#[cfg(test)]
mod cursor_positioning_tests {
    use crate::{AnsiSequenceGenerator, CsiSequence, col, height, row, term_col, term_row};
    use crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_fixtures_vt_100_ansi_conformance::nz;

    #[test]
    fn test_cursor_position_absolute() {
        let seq = AnsiSequenceGenerator::cursor_position(row(5), col(10));
        // row 5 (0-based) = 6 (1-based), col 10 (0-based) = 11 (1-based)
        assert_eq!(
            seq,
            CsiSequence::CursorPosition {
                row: term_row(nz(6)),
                col: term_col(nz(11))
            }
            .to_string()
        );
    }

    #[test]
    fn test_cursor_position_origin() {
        let seq = AnsiSequenceGenerator::cursor_position(row(0), col(0));
        assert_eq!(
            seq,
            CsiSequence::CursorPosition {
                row: term_row(nz(1)),
                col: term_col(nz(1))
            }
            .to_string()
        );
    }

    #[test]
    fn test_cursor_to_column() {
        let seq = AnsiSequenceGenerator::cursor_to_column(col(15));
        assert_eq!(seq, CsiSequence::CursorHorizontalAbsolute(16).to_string());
    }

    #[test]
    fn test_cursor_next_line() {
        let seq = AnsiSequenceGenerator::cursor_next_line(height(3));
        assert_eq!(seq, CsiSequence::CursorNextLine(3).to_string());
    }

    #[test]
    fn test_cursor_previous_line() {
        let seq = AnsiSequenceGenerator::cursor_previous_line(height(2));
        assert_eq!(seq, CsiSequence::CursorPrevLine(2).to_string());
    }
}

#[cfg(test)]
mod screen_clearing_tests {
    use crate::{AnsiSequenceGenerator, CsiSequence, ED_ERASE_ALL, EL_ERASE_ALL,
                EL_ERASE_FROM_START, EL_ERASE_TO_END};

    #[test]
    fn test_clear_screen() {
        let seq = AnsiSequenceGenerator::clear_screen();
        assert_eq!(seq, CsiSequence::EraseDisplay(ED_ERASE_ALL).to_string());
    }

    #[test]
    fn test_clear_current_line() {
        let seq = AnsiSequenceGenerator::clear_current_line();
        assert_eq!(seq, CsiSequence::EraseLine(EL_ERASE_ALL).to_string());
    }

    #[test]
    fn test_clear_to_end_of_line() {
        let seq = AnsiSequenceGenerator::clear_to_end_of_line();
        assert_eq!(seq, CsiSequence::EraseLine(EL_ERASE_TO_END).to_string());
    }

    #[test]
    fn test_clear_to_start_of_line() {
        let seq = AnsiSequenceGenerator::clear_to_start_of_line();
        assert_eq!(seq, CsiSequence::EraseLine(EL_ERASE_FROM_START).to_string());
    }
}

#[cfg(test)]
mod color_tests {
    use crate::{AnsiSequenceGenerator, SgrColorSequence, tui_color};

    #[test]
    fn test_reset_color() {
        let seq = AnsiSequenceGenerator::reset_color();
        assert_eq!(seq, AnsiSequenceGenerator::reset_color());
    }

    // ANSI Basic Colors (0-15) - Widely supported by all terminals
    #[test]
    fn test_fg_ansi_black() {
        let color = tui_color!(black);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Uses extended palette format with colons: SGR 38:5:N
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(0).to_string());
    }

    #[test]
    fn test_fg_ansi_red() {
        let color = tui_color!(red);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // ANSI 1 in extended palette format
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(1).to_string());
    }

    #[test]
    fn test_fg_ansi_green() {
        let color = tui_color!(green);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // ANSI 2 in extended palette format
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(2).to_string());
    }

    #[test]
    fn test_fg_ansi_blue() {
        let color = tui_color!(blue);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // ANSI 4 in extended palette format
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(4).to_string());
    }

    #[test]
    fn test_bg_ansi_black() {
        let color = tui_color!(black);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // Background extended palette format: SGR 48:5:N
        assert_eq!(seq, SgrColorSequence::SetBackgroundAnsi256(0).to_string());
    }

    #[test]
    fn test_bg_ansi_red() {
        let color = tui_color!(red);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // ANSI 1 background in extended palette format
        assert_eq!(seq, SgrColorSequence::SetBackgroundAnsi256(1).to_string());
    }

    #[test]
    fn test_bg_ansi_green() {
        let color = tui_color!(green);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // ANSI 2 background in extended palette format
        assert_eq!(seq, SgrColorSequence::SetBackgroundAnsi256(2).to_string());
    }

    // ANSI Extended Palette (16-255)
    #[test]
    fn test_fg_ansi_extended_palette_16() {
        use crate::AnsiValue;
        let color = crate::TuiColor::Ansi(AnsiValue::new(16));
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Extended colors use colon-separated format for better compatibility
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(16).to_string());
    }

    #[test]
    fn test_fg_ansi_extended_palette_196() {
        use crate::AnsiValue;
        let color = crate::TuiColor::Ansi(AnsiValue::new(196));
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Extended palette index 196 = pure red in 256-color palette
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(196).to_string());
    }

    #[test]
    fn test_bg_ansi_extended_palette_226() {
        use crate::AnsiValue;
        let color = crate::TuiColor::Ansi(AnsiValue::new(226));
        let seq = AnsiSequenceGenerator::bg_color(color);
        // Extended palette index 226 = yellow in 256-color palette
        assert_eq!(seq, SgrColorSequence::SetBackgroundAnsi256(226).to_string());
    }

    // RGB Colors (24-bit true color)
    #[test]
    fn test_fg_rgb_pure_red() {
        let color = tui_color!(255, 0, 0);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // RGB uses 24-bit truecolor with colons: SGR 38:2:R:G:B
        assert_eq!(
            seq,
            SgrColorSequence::SetForegroundRgb(255, 0, 0).to_string()
        );
    }

    #[test]
    fn test_fg_rgb_pure_green() {
        let color = tui_color!(0, 255, 0);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(
            seq,
            SgrColorSequence::SetForegroundRgb(0, 255, 0).to_string()
        );
    }

    #[test]
    fn test_fg_rgb_pure_blue() {
        let color = tui_color!(0, 0, 255);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(
            seq,
            SgrColorSequence::SetForegroundRgb(0, 0, 255).to_string()
        );
    }

    #[test]
    fn test_fg_rgb_orange() {
        let color = tui_color!(255, 165, 0);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(
            seq,
            SgrColorSequence::SetForegroundRgb(255, 165, 0).to_string()
        );
    }

    #[test]
    fn test_bg_rgb_pure_red() {
        let color = tui_color!(255, 0, 0);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // Background RGB uses colon-separated format: SGR 48:2:R:G:B
        assert_eq!(
            seq,
            SgrColorSequence::SetBackgroundRgb(255, 0, 0).to_string()
        );
    }

    #[test]
    fn test_bg_rgb_cyan() {
        let color = tui_color!(0, 255, 255);
        let seq = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(
            seq,
            SgrColorSequence::SetBackgroundRgb(0, 255, 255).to_string()
        );
    }

    #[test]
    fn test_bg_rgb_dark_gray() {
        let color = tui_color!(64, 64, 64);
        let seq = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(
            seq,
            SgrColorSequence::SetBackgroundRgb(64, 64, 64).to_string()
        );
    }

    // Dark ANSI colors (8-15) - Extended basic colors
    #[test]
    fn test_fg_ansi_dark_red() {
        let color = tui_color!(dark_red);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Dark colors use extended palette mode with colons
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(9).to_string());
    }

    #[test]
    fn test_fg_ansi_dark_green() {
        let color = tui_color!(dark_green);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(seq, SgrColorSequence::SetForegroundAnsi256(10).to_string());
    }

    #[test]
    fn test_bg_ansi_dark_blue() {
        let color = tui_color!(dark_blue);
        let seq = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(seq, SgrColorSequence::SetBackgroundAnsi256(12).to_string());
    }
}

#[cfg(test)]
mod cursor_visibility_tests {
    use crate::{AnsiSequenceGenerator, CsiSequence, PrivateModeType};

    #[test]
    fn test_show_cursor() {
        let seq = AnsiSequenceGenerator::show_cursor();
        assert_eq!(
            seq,
            CsiSequence::EnablePrivateMode(PrivateModeType::ShowCursor).to_string()
        );
    }

    #[test]
    fn test_hide_cursor() {
        let seq = AnsiSequenceGenerator::hide_cursor();
        assert_eq!(
            seq,
            CsiSequence::DisablePrivateMode(PrivateModeType::ShowCursor).to_string()
        );
    }
}

#[cfg(test)]
mod terminal_mode_tests {
    use crate::{APPLICATION_MOUSE_TRACKING, AnsiSequenceGenerator, CsiSequence,
                PrivateModeType, SGR_MOUSE_MODE, URXVT_MOUSE_EXTENSION};

    #[test]
    fn test_enter_alternate_screen() {
        let seq = AnsiSequenceGenerator::enter_alternate_screen();
        assert_eq!(
            seq,
            CsiSequence::EnablePrivateMode(PrivateModeType::AlternateScreenBuffer)
                .to_string()
        );
    }

    #[test]
    fn test_exit_alternate_screen() {
        let seq = AnsiSequenceGenerator::exit_alternate_screen();
        assert_eq!(
            seq,
            CsiSequence::DisablePrivateMode(PrivateModeType::AlternateScreenBuffer)
                .to_string()
        );
    }

    #[test]
    fn test_enable_mouse_tracking() {
        let seq = AnsiSequenceGenerator::enable_mouse_tracking();
        // Order: SGR Mode (1006), Application Mouse Tracking (1003), Mouse Mode Extension
        // (1015)
        let expected = format!(
            "{}{}{}",
            CsiSequence::EnablePrivateMode(PrivateModeType::Other(SGR_MOUSE_MODE)),
            CsiSequence::EnablePrivateMode(PrivateModeType::Other(
                APPLICATION_MOUSE_TRACKING
            )),
            CsiSequence::EnablePrivateMode(PrivateModeType::Other(URXVT_MOUSE_EXTENSION))
        );
        assert_eq!(seq, expected);
    }

    #[test]
    fn test_disable_mouse_tracking() {
        let seq = AnsiSequenceGenerator::disable_mouse_tracking();
        // Order: SGR Mode (1006), Application Mouse Tracking (1003), Mouse Mode Extension
        // (1015)
        let expected = format!(
            "{}{}{}",
            CsiSequence::DisablePrivateMode(PrivateModeType::Other(SGR_MOUSE_MODE)),
            CsiSequence::DisablePrivateMode(PrivateModeType::Other(
                APPLICATION_MOUSE_TRACKING
            )),
            CsiSequence::DisablePrivateMode(PrivateModeType::Other(
                URXVT_MOUSE_EXTENSION
            ))
        );
        assert_eq!(seq, expected);
    }
}
