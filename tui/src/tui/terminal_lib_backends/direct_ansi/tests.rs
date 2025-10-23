// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for [`AnsiSequenceGenerator`]
//!
//! Tests all static methods to verify correct ANSI escape sequence generation.

#[cfg(test)]
mod cursor_positioning_tests {
    use crate::{col, height, row,
                terminal_lib_backends::direct_ansi::AnsiSequenceGenerator};

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
    use crate::terminal_lib_backends::direct_ansi::AnsiSequenceGenerator;

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
    use crate::terminal_lib_backends::direct_ansi::AnsiSequenceGenerator;

    #[test]
    fn test_reset_color() {
        let seq = AnsiSequenceGenerator::reset_color();
        assert_eq!(seq, "\x1b[0m");
    }

    // TODO: Add color tests once TuiColor types are clarified
}

#[cfg(test)]
mod cursor_visibility_tests {
    use crate::terminal_lib_backends::direct_ansi::AnsiSequenceGenerator;

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
    use crate::terminal_lib_backends::direct_ansi::AnsiSequenceGenerator;

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
