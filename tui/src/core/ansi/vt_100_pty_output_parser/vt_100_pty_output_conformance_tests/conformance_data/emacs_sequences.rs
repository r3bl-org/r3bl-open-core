// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Emacs editor sequence patterns for cross-editor compatibility testing.
//!
//! This module provides sequences that simulate Emacs terminal behavior,
//! allowing tests to verify compatibility with different text editor patterns.
//! Useful for ensuring ANSI parser works correctly with various editor implementations.
//!
//! ## Emacs Terminal Behavior
//!
//! - Mode lines typically appear at bottom with reverse video
//! - Uses different status indicators than vim
//! - Often employs more conservative styling

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{ANSIBasicColor, EraseDisplayMode, EraseLineMode, SgrCode,
            core::ansi::vt_100_pty_output_parser::CsiSequence, term_col, term_row};

/// Generate Emacs-style mode line display.
///
/// **Emacs Convention**: Mode line appears at bottom with distinctive formatting
/// and shows buffer information, major mode, and other status indicators.
#[must_use]
pub fn emacs_mode_line() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        // Move to bottom line.
        CsiSequence::CursorPosition {
            row: term_row(nz(25)),
            col: term_col(nz(1))
        },
        // Cyan background for mode line.
        SgrCode::BackgroundBasic(ANSIBasicColor::Cyan),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        "-UUU:",
        SgrCode::ForegroundBasic(ANSIBasicColor::DarkGray),
        "F1  ",
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        "main.rs",
        "   ",
        SgrCode::Bold,
        "(Rust)",
        SgrCode::Reset,
        SgrCode::BackgroundBasic(ANSIBasicColor::Cyan),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        "  L1  C1  All (2,15) [Git:main] 15:30",
        SgrCode::Reset,
        // Clear to end of line.
        CsiSequence::EraseLine(EraseLineMode::FromCursorToEnd)
    )
}

/// Generate Emacs-style minibuffer prompt.
///
/// **Emacs Convention**: Minibuffer appears at bottom for commands and input
#[must_use]
pub fn emacs_minibuffer_prompt(prompt: &str) -> String {
    format!(
        "{}{}{}{}{}",
        // Move to bottom line.
        CsiSequence::CursorPosition {
            row: term_row(nz(10)),
            col: term_col(nz(1))
        },
        // Clear the line first.
        CsiSequence::EraseLine(EraseLineMode::EntireLine),
        // Bold prompt.
        SgrCode::Bold,
        prompt,
        SgrCode::Reset
    )
}

/// Generate Emacs-style buffer switching display.
///
/// Shows multiple buffers with switching indicators,
/// simulating M-x list-buffers functionality.
#[must_use]
pub fn emacs_buffer_list() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}",
        // Clear screen and move to top.
        CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen),
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        // Header.
        SgrCode::Bold,
        " MR Buffer",
        SgrCode::Reset,
        "\n",
        // Active buffer.
        " *  main.rs\n",
        // Background buffer.
        "    README.md\n",
        // Modified buffer.
        SgrCode::ForegroundBasic(ANSIBasicColor::Red),
        " %* config.toml",
        SgrCode::Reset,
        "\n"
    )
}
