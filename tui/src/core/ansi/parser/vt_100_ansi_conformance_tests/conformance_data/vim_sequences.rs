// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Vim editor sequence patterns for real-world terminal application testing.
//!
//! This module provides sequences that mirror actual Vim editor behavior,
//! including status lines, visual mode highlighting, and screen management.
//! These patterns test complex combinations of cursor movement, styling,
//! and screen manipulation that real applications use.
//!
//! ## Real-World Context
//!
//! Vim is one of the most sophisticated terminal applications in terms of
//! ANSI sequence usage. It employs advanced cursor management, complex
//! styling patterns, and efficient screen updates that stress-test terminal
//! parsers and buffer management.

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{ANSIBasicColor, EscSequence, LengthOps, SgrCode,
            core::ansi::parser::CsiSequence, height, term_col, term_row};
use std::num::NonZeroU16;

/// Vim status line pattern with mode indicator.
///
/// **Real-world pattern**: Vim displays mode information (INSERT, VISUAL, etc.)
/// in a status line at the bottom of the screen using reverse video highlighting.
///
/// This sequence demonstrates:
/// - Cursor save/restore for temporary operations
/// - Absolute positioning to bottom of screen
/// - Reverse video styling for highlighting
/// - Proper style reset to avoid affecting subsequent output
///
/// # Arguments
/// * `mode` - Mode string to display (e.g., "INSERT", "VISUAL", "NORMAL")
/// * `status_row` - Row for status line (typically bottom row of terminal)
#[must_use]
pub fn vim_status_line(mode: &str, status_row: NonZeroU16) -> String {
    format!(
        "{}{}{}-- {} --{}{}",
        EscSequence::SaveCursor,
        CsiSequence::CursorPosition {
            row: term_row(status_row),
            col: term_col(nz(1))
        },
        SgrCode::Invert, // Reverse video for highlighting
        mode,
        SgrCode::Reset,
        EscSequence::RestoreCursor
    )
}

/// Vim visual selection highlighting pattern.
///
/// **Real-world pattern**: When text is selected in Visual mode, Vim highlights
/// the selected region with background color, often spanning multiple lines.
///
/// This sequence demonstrates:
/// - Multi-line selection highlighting
/// - Precise cursor positioning for block operations
/// - Background color application across regions
/// - Complex coordinate calculations
///
/// # Arguments
/// * `start_row` - Starting row of selection (1-based)
/// * `start_col` - Starting column of selection (1-based)
/// * `end_row` - Ending row of selection (1-based)
/// * `end_col` - Ending column of selection (1-based)
#[must_use]
pub fn vim_visual_selection(
    start_row: u16,
    start_col: u16,
    end_row: u16,
    end_col: u16,
) -> String {
    let mut sequence = String::new();

    for row in start_row..=end_row {
        let (col_start, col_end) = if row == start_row && row == end_row {
            // Single line selection.
            (start_col, end_col)
        } else if row == start_row {
            // First line of multi-line selection.
            (start_col, 80) // Assume 80-column terminal
        } else if row == end_row {
            // Last line of multi-line selection.
            (1, end_col)
        } else {
            // Middle lines of multi-line selection.
            (1, 80)
        };

        // Move to start of selection on this line.
        let row_nz = nz(row);
        let col_nz = nz(col_start);
        sequence.push_str(
            &CsiSequence::CursorPosition {
                row: term_row(row_nz),
                col: term_col(col_nz),
            }
            .to_string(),
        );

        // Apply selection highlighting.
        sequence.push_str(&SgrCode::BackgroundBasic(ANSIBasicColor::Blue).to_string());

        // Fill the selected area (simplified - would normally preserve existing text)
        let selection_width = col_end - col_start + 1;
        sequence.push_str(&" ".repeat(selection_width as usize));
    }

    // Reset styling.
    sequence.push_str(&SgrCode::Reset.to_string());
    sequence
}

/// Vim command line pattern.
///
/// **Real-world pattern**: When entering commands (`:`, `/`, `?`), Vim displays
/// a command line at the bottom of the screen and positions the cursor for input.
///
/// # Arguments
/// * `command_char` - Command character (`:`, `/`, `?`)
/// * `command_row` - Row for command line (typically bottom row)
#[must_use]
pub fn vim_command_line(command_char: char, command_row: NonZeroU16) -> String {
    format!(
        "{}{}{}",
        EscSequence::SaveCursor,
        CsiSequence::CursorPosition {
            row: term_row(command_row),
            col: term_col(nz(1))
        },
        command_char
    )
}

/// Vim screen clear and redraw pattern.
///
/// **Real-world pattern**: Vim frequently clears and redraws the screen
/// for operations like `:clear`, `:redraw`, or when switching buffers.
///
/// This demonstrates the common pattern of:
/// 1. Clear entire screen
/// 2. Position cursor at home
/// 3. Optionally display content
#[must_use]
pub fn vim_clear_and_redraw() -> String {
    format!(
        "{}{}",
        CsiSequence::EraseDisplay(2), // Clear screen
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        }  // Home cursor
    )
}

/// Vim line numbering pattern.
///
/// **Real-world pattern**: When `:set number` is enabled, Vim displays
/// line numbers in a left margin with different styling.
///
/// # Arguments
/// * `line_num` - Line number to display
/// * `line_row` - Row where line number appears
/// * `content` - Line content to display after number
#[must_use]
pub fn vim_line_with_number(
    line_num: u16,
    line_row: NonZeroU16,
    content: &str,
) -> String {
    format!(
        "{}{}{:4} {}{}{}",
        CsiSequence::CursorPosition {
            row: term_row(line_row),
            col: term_col(nz(1))
        },
        SgrCode::ForegroundBasic(ANSIBasicColor::Yellow), // Dim line numbers
        line_num,                                         /* Right-aligned 4-digit
                                                           * line number */
        SgrCode::Reset,
        content,
        SgrCode::Reset
    )
}

/// Vim search highlight pattern.
///
/// **Real-world pattern**: When searching with `/` or `?`, Vim highlights
/// matching text with background color and may show multiple matches.
///
/// # Arguments
/// * `row` - Row containing the match
/// * `col` - Column where match starts
/// * `match_text` - Text that matches the search
#[must_use]
pub fn vim_search_highlight(
    row: NonZeroU16,
    col: NonZeroU16,
    match_text: &str,
) -> String {
    format!(
        "{}{}{}{}{}",
        CsiSequence::CursorPosition {
            row: term_row(row),
            col: term_col(col)
        },
        SgrCode::BackgroundBasic(ANSIBasicColor::Yellow),
        SgrCode::ForegroundBasic(ANSIBasicColor::Black),
        match_text,
        SgrCode::Reset
    )
}

/// Vim error message pattern.
///
/// **Real-world pattern**: Vim displays error messages at the bottom of
/// the screen with red coloring and often uses the bell character.
///
/// # Arguments
/// * `error_msg` - Error message to display
/// * `error_row` - Row for error display (typically bottom row)
#[must_use]
pub fn vim_error_message(error_msg: &str, error_row: NonZeroU16) -> String {
    format!(
        "{}{}{}{}{}{}{}",
        EscSequence::SaveCursor,
        CsiSequence::CursorPosition {
            row: term_row(error_row),
            col: term_col(nz(1))
        },
        SgrCode::ForegroundBasic(ANSIBasicColor::Red),
        SgrCode::Bold,
        error_msg,
        SgrCode::Reset,
        EscSequence::RestoreCursor
    )
}

/// Vim tab completion menu pattern.
///
/// **Real-world pattern**: When using tab completion in command mode,
/// Vim shows a menu of possible completions above the command line.
///
/// # Arguments
/// * `completions` - List of completion options
/// * `start_row` - Row to start displaying completions
///
/// # Panics
/// May panic if arithmetic operations overflow during sequence generation.
#[must_use]
pub fn vim_completion_menu(completions: &[&str], start_row: NonZeroU16) -> String {
    let mut sequence = String::new();

    sequence.push_str(&EscSequence::SaveCursor.to_string());

    // Display each completion option.
    for (i, completion) in completions.iter().enumerate() {
        let row_offset = height(i).clamp_to_max(u16::MAX).as_u16();
        let row_nz = NonZeroU16::new(start_row.get() + row_offset)
            .expect("start_row + row_offset is always >= 1");
        sequence.push_str(
            &CsiSequence::CursorPosition {
                row: term_row(row_nz),
                col: term_col(nz(1)),
            }
            .to_string(),
        );

        // Highlight first option.
        if i == 0 {
            sequence.push_str(&SgrCode::Invert.to_string());
        }

        sequence.push_str(completion);

        if i == 0 {
            sequence.push_str(&SgrCode::Reset.to_string());
        }
    }

    sequence.push_str(&EscSequence::RestoreCursor.to_string());
    sequence
}

/// Vim syntax highlighting pattern.
///
/// **Real-world pattern**: Vim applies different colors to different
/// syntax elements (keywords, strings, comments, etc.).
///
/// This creates a sample of syntax-highlighted code to test
/// multiple color changes in sequence.
#[must_use]
pub fn vim_syntax_highlighting() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        // Keyword in blue.
        SgrCode::ForegroundBasic(ANSIBasicColor::Blue),
        SgrCode::Bold,
        "fn",
        SgrCode::Reset,
        " ",
        // Function name in default color.
        "main",
        "() {\n    ",
        // String in green.
        SgrCode::ForegroundBasic(ANSIBasicColor::Green),
        "\"Hello, World!\"",
        SgrCode::Reset,
        ";\n    ",
        // Comment in gray.
        SgrCode::ForegroundBasic(ANSIBasicColor::DarkGray),
        "// This is a comment",
        SgrCode::Reset,
        "\n}",
        SgrCode::Reset
    )
}
