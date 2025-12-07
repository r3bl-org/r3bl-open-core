// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tmux terminal multiplexer sequence patterns for testing complex screen management.
//!
//! This module provides sequences that simulate tmux behavior including
//! pane management, window switching, and status bar display. These patterns
//! are essential for testing terminal applications in multiplexed environments.
//!
//! ## Tmux Terminal Behavior
//!
//! - Status bar typically at bottom with session/window information
//! - Pane borders use line drawing characters
//! - Window indicators show activity and current status
//! - Copy mode uses reverse video for selection

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{ANSIBasicColor, EraseDisplayMode, EraseLineMode, SgrCode,
            core::ansi::vt_100_pty_output_parser::CsiSequence, term_col, term_row};

/// Generate tmux-style status bar display.
///
/// **Tmux Convention**: Status bar shows session info, window list, and system info
/// with distinctive green background and configurable format.
#[must_use]
pub fn tmux_status_bar() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        // Move to bottom line.
        CsiSequence::CursorPosition {
            row: term_row(nz(25)),
            col: term_col(nz(1))
        },
        // Green background for status bar.
        SgrCode::BackgroundBasic(ANSIBasicColor::Green),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        "[",
        SgrCode::Bold,
        "main",
        SgrCode::Reset,
        SgrCode::BackgroundBasic(ANSIBasicColor::Green),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        "] ",
        // Current window indicator.
        SgrCode::Invert,
        "0:zsh",
        SgrCode::Reset,
        SgrCode::BackgroundBasic(ANSIBasicColor::Green),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        " 1:vim 2:git",
        // Right side with time/host.
        " \"r3bl-host\" 15:30:45",
        SgrCode::Reset,
        // Clear to end of line.
        CsiSequence::EraseLine(EraseLineMode::FromCursorToEnd)
    )
}

/// Generate tmux pane splitting display.
///
/// **Tmux Convention**: Panes are separated by borders using line drawing characters
/// and each pane maintains its own cursor position.
#[must_use]
pub fn tmux_pane_split_horizontal() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        // Clear screen.
        CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen),
        // Draw top pane content.
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        "Top pane content",
        // Draw horizontal border.
        CsiSequence::CursorPosition {
            row: term_row(nz(5)),
            col: term_col(nz(1))
        },
        "─".repeat(10),
        // Draw bottom pane content.
        CsiSequence::CursorPosition {
            row: term_row(nz(6)),
            col: term_col(nz(1))
        },
        "Bottom pane",
        // Show pane indicators.
        CsiSequence::CursorPosition {
            row: term_row(nz(5)),
            col: term_col(nz(1))
        },
        SgrCode::ForegroundBasic(ANSIBasicColor::Blue),
        "0",
        SgrCode::Reset,
        CsiSequence::CursorPosition {
            row: term_row(nz(6)),
            col: term_col(nz(1))
        },
        SgrCode::ForegroundBasic(ANSIBasicColor::Blue),
        "1",
        SgrCode::Reset
    )
}

/// Generate tmux copy mode selection display.
///
/// **Tmux Convention**: Copy mode uses reverse video to highlight selected text
/// and shows copy mode indicator in status.
#[must_use]
pub fn tmux_copy_mode_selection(
    start_row: u16,
    start_col: u16,
    _end_row: u16,
    _end_col: u16,
) -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        // Move to selection start.
        CsiSequence::CursorPosition {
            row: term_row(nz(start_row)),
            col: term_col(nz(start_col))
        },
        // Start reverse video for selection.
        SgrCode::Invert,
        "selected",
        SgrCode::Reset,
        // Show copy mode indicator in status.
        CsiSequence::CursorPosition {
            row: term_row(nz(10)),
            col: term_col(nz(1))
        },
        SgrCode::BackgroundBasic(ANSIBasicColor::Yellow),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        "[Copy Mode]",
        SgrCode::Reset,
        // Clear to end of status line.
        CsiSequence::EraseLine(EraseLineMode::FromCursorToEnd)
    )
}

/// Generate tmux session switching display.
///
/// Shows session list with current session highlighted,
/// simulating tmux's session management interface.
#[must_use]
pub fn tmux_session_list() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        // Clear screen. and show session list
        CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen),
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        SgrCode::Bold,
        "Sessions:",
        SgrCode::Reset,
        "\n",
        // Current session (highlighted)
        SgrCode::Invert,
        "  main: 2 windows",
        SgrCode::Reset,
        "\n",
        // Other sessions.
        "  work: 1 window\n",
        SgrCode::ForegroundBasic(ANSIBasicColor::DarkGray),
        "  old: 0 windows",
        SgrCode::Reset,
        "\n"
    )
}
