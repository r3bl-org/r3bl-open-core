// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for [`AnsiSequenceGenerator`]
//!
//! Tests all static methods to verify correct ANSI escape sequence generation.

#[cfg(test)]
mod cursor_positioning_tests {
    use crate::{col, height, row, AnsiSequenceGenerator};

    #[test]
    fn test_cursor_position_absolute() {
        let seq = AnsiSequenceGenerator::cursor_position(row(5), col(10));
        // row 5 (0-based) = 6 (1-based), col 10 (0-based) = 11 (1-based)
        assert_eq!(seq, "\x1b[6;11H");
    }

    #[test]
    fn test_cursor_position_origin() {
        let seq = AnsiSequenceGenerator::cursor_position(row(0), col(0));
        assert_eq!(seq, "\x1b[1;1H");
    }

    #[test]
    fn test_cursor_to_column() {
        let seq = AnsiSequenceGenerator::cursor_to_column(col(15));
        assert_eq!(seq, "\x1b[16G");
    }

    #[test]
    fn test_cursor_next_line() {
        let seq = AnsiSequenceGenerator::cursor_next_line(height(3));
        assert_eq!(seq, "\x1b[3E");
    }

    #[test]
    fn test_cursor_previous_line() {
        let seq = AnsiSequenceGenerator::cursor_previous_line(height(2));
        assert_eq!(seq, "\x1b[2F");
    }
}

#[cfg(test)]
mod screen_clearing_tests {
    use crate::AnsiSequenceGenerator;

    #[test]
    fn test_clear_screen() {
        let seq = AnsiSequenceGenerator::clear_screen();
        assert_eq!(seq, "\x1b[2J");
    }

    #[test]
    fn test_clear_current_line() {
        let seq = AnsiSequenceGenerator::clear_current_line();
        assert_eq!(seq, "\x1b[2K");
    }

    #[test]
    fn test_clear_to_end_of_line() {
        let seq = AnsiSequenceGenerator::clear_to_end_of_line();
        assert_eq!(seq, "\x1b[0K");
    }

    #[test]
    fn test_clear_to_start_of_line() {
        let seq = AnsiSequenceGenerator::clear_to_start_of_line();
        assert_eq!(seq, "\x1b[1K");
    }
}

#[cfg(test)]
mod color_tests {
    use crate::{tui_color, AnsiSequenceGenerator};

    #[test]
    fn test_reset_color() {
        let seq = AnsiSequenceGenerator::reset_color();
        assert_eq!(seq, "\x1b[0m");
    }

    // ANSI Basic Colors (0-15) - Widely supported by all terminals
    #[test]
    fn test_fg_ansi_black() {
        let color = tui_color!(black);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Uses extended palette format with colons: SGR 38:5:N
        assert_eq!(seq, "\x1b[38:5:0m");
    }

    #[test]
    fn test_fg_ansi_red() {
        let color = tui_color!(red);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // ANSI 1 in extended palette format
        assert_eq!(seq, "\x1b[38:5:1m");
    }

    #[test]
    fn test_fg_ansi_green() {
        let color = tui_color!(green);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // ANSI 2 in extended palette format
        assert_eq!(seq, "\x1b[38:5:2m");
    }

    #[test]
    fn test_fg_ansi_blue() {
        let color = tui_color!(blue);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // ANSI 4 in extended palette format
        assert_eq!(seq, "\x1b[38:5:4m");
    }

    #[test]
    fn test_bg_ansi_black() {
        let color = tui_color!(black);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // Background extended palette format: SGR 48:5:N
        assert_eq!(seq, "\x1b[48:5:0m");
    }

    #[test]
    fn test_bg_ansi_red() {
        let color = tui_color!(red);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // ANSI 1 background in extended palette format
        assert_eq!(seq, "\x1b[48:5:1m");
    }

    #[test]
    fn test_bg_ansi_green() {
        let color = tui_color!(green);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // ANSI 2 background in extended palette format
        assert_eq!(seq, "\x1b[48:5:2m");
    }

    // ANSI Extended Palette (16-255)
    #[test]
    fn test_fg_ansi_extended_palette_16() {
        use crate::AnsiValue;
        let color = crate::TuiColor::Ansi(AnsiValue::new(16));
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Extended colors use colon-separated format for better compatibility
        assert_eq!(seq, "\x1b[38:5:16m");
    }

    #[test]
    fn test_fg_ansi_extended_palette_196() {
        use crate::AnsiValue;
        let color = crate::TuiColor::Ansi(AnsiValue::new(196));
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Extended palette index 196 = pure red in 256-color palette
        assert_eq!(seq, "\x1b[38:5:196m");
    }

    #[test]
    fn test_bg_ansi_extended_palette_226() {
        use crate::AnsiValue;
        let color = crate::TuiColor::Ansi(AnsiValue::new(226));
        let seq = AnsiSequenceGenerator::bg_color(color);
        // Extended palette index 226 = yellow in 256-color palette
        assert_eq!(seq, "\x1b[48:5:226m");
    }

    // RGB Colors (24-bit true color)
    #[test]
    fn test_fg_rgb_pure_red() {
        let color = tui_color!(255, 0, 0);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // RGB uses 24-bit truecolor with colons: SGR 38:2:R:G:B
        assert_eq!(seq, "\x1b[38:2:255:0:0m");
    }

    #[test]
    fn test_fg_rgb_pure_green() {
        let color = tui_color!(0, 255, 0);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(seq, "\x1b[38:2:0:255:0m");
    }

    #[test]
    fn test_fg_rgb_pure_blue() {
        let color = tui_color!(0, 0, 255);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(seq, "\x1b[38:2:0:0:255m");
    }

    #[test]
    fn test_fg_rgb_orange() {
        let color = tui_color!(255, 165, 0);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(seq, "\x1b[38:2:255:165:0m");
    }

    #[test]
    fn test_bg_rgb_pure_red() {
        let color = tui_color!(255, 0, 0);
        let seq = AnsiSequenceGenerator::bg_color(color);
        // Background RGB uses colon-separated format: SGR 48:2:R:G:B
        assert_eq!(seq, "\x1b[48:2:255:0:0m");
    }

    #[test]
    fn test_bg_rgb_cyan() {
        let color = tui_color!(0, 255, 255);
        let seq = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(seq, "\x1b[48:2:0:255:255m");
    }

    #[test]
    fn test_bg_rgb_dark_gray() {
        let color = tui_color!(64, 64, 64);
        let seq = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(seq, "\x1b[48:2:64:64:64m");
    }

    // Dark ANSI colors (8-15) - Extended basic colors
    #[test]
    fn test_fg_ansi_dark_red() {
        let color = tui_color!(dark_red);
        let seq = AnsiSequenceGenerator::fg_color(color);
        // Dark colors use extended palette mode with colons
        assert_eq!(seq, "\x1b[38:5:9m");
    }

    #[test]
    fn test_fg_ansi_dark_green() {
        let color = tui_color!(dark_green);
        let seq = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(seq, "\x1b[38:5:10m");
    }

    #[test]
    fn test_bg_ansi_dark_blue() {
        let color = tui_color!(dark_blue);
        let seq = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(seq, "\x1b[48:5:12m");
    }
}

#[cfg(test)]
mod cursor_visibility_tests {
    use crate::AnsiSequenceGenerator;

    #[test]
    fn test_show_cursor() {
        let seq = AnsiSequenceGenerator::show_cursor();
        assert_eq!(seq, "\x1b[?25h");
    }

    #[test]
    fn test_hide_cursor() {
        let seq = AnsiSequenceGenerator::hide_cursor();
        assert_eq!(seq, "\x1b[?25l");
    }
}

#[cfg(test)]
mod terminal_mode_tests {
    use crate::AnsiSequenceGenerator;

    #[test]
    fn test_enter_alternate_screen() {
        let seq = AnsiSequenceGenerator::enter_alternate_screen();
        assert_eq!(seq, "\x1b[?1049h");
    }

    #[test]
    fn test_exit_alternate_screen() {
        let seq = AnsiSequenceGenerator::exit_alternate_screen();
        assert_eq!(seq, "\x1b[?1049l");
    }

    #[test]
    fn test_enable_mouse_tracking() {
        let seq = AnsiSequenceGenerator::enable_mouse_tracking();
        // Order: SGR Mode (1006), Application Mouse Tracking (1003), Mouse Mode Extension
        // (1015)
        assert_eq!(seq, "\x1b[?1006h\x1b[?1003h\x1b[?1015h");
    }

    #[test]
    fn test_disable_mouse_tracking() {
        let seq = AnsiSequenceGenerator::disable_mouse_tracking();
        // Order: SGR Mode (1006), Application Mouse Tracking (1003), Mouse Mode Extension
        // (1015)
        assert_eq!(seq, "\x1b[?1006l\x1b[?1003l\x1b[?1015l");
    }
}
