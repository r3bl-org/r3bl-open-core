// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI escape sequence generator for terminal operations. See [`AnsiSequenceGenerator`].

use crate::{ColIndex, ColorTarget, RowHeight, RowIndex, SgrColorSequence, TuiColor,
            TuiStyle,
            core::{ansi::{constants::{APPLICATION_MOUSE_TRACKING, BRACKETED_PASTE_MODE,
                                      CSI_PARAM_SEPARATOR, CSI_START, ED_ERASE_ALL,
                                      EL_ERASE_ALL, EL_ERASE_FROM_START, EL_ERASE_TO_END,
                                      SGR_BOLD, SGR_DIM, SGR_ITALIC, SGR_SET_GRAPHICS,
                                      SGR_STRIKETHROUGH, SGR_UNDERLINE, SGR_MOUSE_MODE,
                                      URXVT_MOUSE_EXTENSION},
                          vt_100_pty_output_parser::{CsiSequence, PrivateModeType}},
                   coordinates::{TermCol, TermRow}}};

/// Generates ANSI escape sequence strings for terminal operations.
///
/// This module generates raw ANSI escape sequence bytes for terminal operations using
/// semantic types and traits for type-safe sequence generation. Works in conjunction with
/// [`vt_100_pty_output_parser`] for bidirectional ANSI handling.
///
/// # Design Philosophy
///
/// - **No raw format! calls**: All sequences are generated using typed enums +
///   [`FastStringify`]
/// - **Type-safe sequences**: Uses [`CsiSequence`], [`SgrColorSequence`], and other
///   sequence enums
/// - **Reuses infrastructure**: Leverages existing ANSI types and constants
/// - **Infallible generation**: Exhaustive pattern matching ensures valid output
/// - **1-based indexing**: Automatically converts 0-based indices to 1-based ANSI
///
/// [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
///
/// # Reference Implementation Pattern
///
/// Instead of raw format strings (❌ DON'T DO THIS):
/// ```rust
/// # use r3bl_tui::{row, col};
/// let row_idx = row(5);
/// let col_idx = col(10);
/// // Avoid this approach:
/// let _seq: String = format!("\x1b[{};{}H", row_idx.as_usize() + 1, col_idx.as_usize() + 1);
/// ```
///
/// We now use semantic enums (✅ CURRENT APPROACH):
/// ```rust
/// # use r3bl_tui::{row, col, AnsiSequenceGenerator};
/// let row_idx = row(5);
/// let col_idx = col(10);
/// let seq = AnsiSequenceGenerator::cursor_position(row_idx, col_idx);
/// assert_eq!(seq, "\x1b[6;11H");  // row 5 → 6, col 10 → 11 (1-based)
/// ```
///
/// This achieves:
/// - **Semantic clarity**: Intent is explicit (method name shows what we're doing)
/// - **Type safety**: Only valid sequences can be constructed via enums
/// - **Infallibility**: `FastStringify` guarantees valid ANSI
/// - **Consistency**: Matches test infrastructure patterns
/// - **No allocation waste**: Returns String, avoiding [`String`]→[`Vec<u8>`] conversion
///
/// # Return Types
///
/// All methods return [`String`] containing raw ANSI escape sequence bytes. Callers can:
/// - Use directly if a [`String`] is needed
/// - Call `.into_bytes()` to get [`Vec<u8>`] for writing to output devices
/// - Use `&sequence` to get [`&str`] for slice operations
///
/// This struct has no state; it's a collection of static methods. State tracking (cursor
/// position, current colors) is handled by external implementations.
///
/// [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
/// [`FastStringify`]: crate::core::common::fast_stringify::FastStringify
/// [`CsiSequence`]: crate::CsiSequence
/// [`SgrColorSequence`]: crate::SgrColorSequence
#[derive(Debug)]
pub struct AnsiSequenceGenerator;

impl AnsiSequenceGenerator {
    // ==================== Cursor Movement ====================

    /// Generate absolute cursor positioning
    /// CSI `<row>;<col>H` (1-based indexing)
    #[must_use]
    pub fn cursor_position(row: RowIndex, col: ColIndex) -> String {
        CsiSequence::CursorPosition {
            row: TermRow::from_zero_based(row),
            col: TermCol::from_zero_based(col),
        }
        .to_string()
    }

    /// Generate cursor to column
    /// CSI <col>G (1-based)
    #[must_use]
    pub fn cursor_to_column(col: ColIndex) -> String {
        let term_col = TermCol::from_zero_based(col);
        CsiSequence::CursorHorizontalAbsolute(term_col.as_u16()).to_string()
    }

    /// Generate cursor next line
    /// CSI `<n>E`
    #[must_use]
    pub fn cursor_next_line(rows: RowHeight) -> String {
        CsiSequence::CursorNextLine(rows.as_u16()).to_string()
    }

    /// Generate cursor previous line
    /// CSI `<n>F`
    #[must_use]
    pub fn cursor_previous_line(rows: RowHeight) -> String {
        CsiSequence::CursorPrevLine(rows.as_u16()).to_string()
    }

    // ==================== Screen Clearing ====================

    /// Clear entire screen
    /// CSI 2J (Erase Display: 2 = entire display)
    #[must_use]
    pub fn clear_screen() -> String {
        CsiSequence::EraseDisplay(ED_ERASE_ALL).to_string()
    }

    /// Clear current line
    /// CSI 2K (Erase Line: 2 = entire line)
    #[must_use]
    pub fn clear_current_line() -> String {
        CsiSequence::EraseLine(EL_ERASE_ALL).to_string()
    }

    /// Clear to end of line
    /// CSI 0K (Erase Line: 0 = cursor to end)
    #[must_use]
    pub fn clear_to_end_of_line() -> String {
        CsiSequence::EraseLine(EL_ERASE_TO_END).to_string()
    }

    /// Clear to start of line
    /// CSI 1K (Erase Line: 1 = start to cursor)
    #[must_use]
    pub fn clear_to_start_of_line() -> String {
        CsiSequence::EraseLine(EL_ERASE_FROM_START).to_string()
    }

    // ==================== Color Operations ====================

    /// Generate foreground color sequence
    #[must_use]
    pub fn fg_color(color: TuiColor) -> String {
        let seq: SgrColorSequence = (color, ColorTarget::Foreground).into();
        seq.to_string()
    }

    /// Generate background color sequence
    #[must_use]
    pub fn bg_color(color: TuiColor) -> String {
        let seq: SgrColorSequence = (color, ColorTarget::Background).into();
        seq.to_string()
    }

    /// Generate text attribute sequences (bold, italic, underline, etc.)
    /// Uses semantic SGR codes from the [`vt_100_pty_output_parser`] infrastructure
    ///
    /// [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
    #[must_use]
    pub fn text_attributes(style: &TuiStyle) -> String {
        // Build SGR sequence with all applicable attributes
        let mut codes: Vec<u16> = Vec::new();

        if style.attribs.bold.is_some() {
            codes.push(SGR_BOLD);
        }
        if style.attribs.dim.is_some() {
            codes.push(SGR_DIM);
        }
        if style.attribs.italic.is_some() {
            codes.push(SGR_ITALIC);
        }
        if style.attribs.underline.is_some() {
            codes.push(SGR_UNDERLINE);
        }
        if style.attribs.strikethrough.is_some() {
            codes.push(SGR_STRIKETHROUGH);
        }

        // If no attributes, return empty string
        if codes.is_empty() {
            return String::new();
        }

        // Build the SGR sequence using the constants
        let mut result = CSI_START.to_string();
        for (i, code) in codes.iter().enumerate() {
            if i > 0 {
                result.push(CSI_PARAM_SEPARATOR);
            }
            result.push_str(&code.to_string());
        }
        result.push(SGR_SET_GRAPHICS);

        result
    }

    /// Reset all colors and attributes to default
    /// CSI 0m (SGR Reset)
    #[must_use]
    pub fn reset_color() -> String { format!("{CSI_START}0m") }

    // ==================== Cursor Visibility ====================

    /// Show cursor
    /// CSI ?25h (DECTCEM: DEC Text Cursor Enable Mode = set)
    #[must_use]
    pub fn show_cursor() -> String {
        CsiSequence::EnablePrivateMode(PrivateModeType::ShowCursor).to_string()
    }

    /// Hide cursor
    /// CSI ?25l (DECTCEM = reset)
    #[must_use]
    pub fn hide_cursor() -> String {
        CsiSequence::DisablePrivateMode(PrivateModeType::ShowCursor).to_string()
    }

    // ==================== Cursor Save/Restore ====================

    /// Save cursor position
    /// CSI s (DECSC: Save Cursor)
    #[must_use]
    pub fn save_cursor_position() -> String { CsiSequence::SaveCursor.to_string() }

    /// Restore cursor position
    /// CSI u (DECRC: Restore Cursor)
    #[must_use]
    pub fn restore_cursor_position() -> String { CsiSequence::RestoreCursor.to_string() }

    // ==================== Terminal Modes ====================

    /// Enter alternate screen buffer
    /// CSI ?1049h (`AlternateScreenBuffer`)
    #[must_use]
    pub fn enter_alternate_screen() -> String {
        CsiSequence::EnablePrivateMode(PrivateModeType::AlternateScreenBuffer).to_string()
    }

    /// Exit alternate screen buffer
    /// CSI ?1049l (`AlternateScreenBuffer`)
    #[must_use]
    pub fn exit_alternate_screen() -> String {
        CsiSequence::DisablePrivateMode(PrivateModeType::AlternateScreenBuffer)
            .to_string()
    }

    /// Enable mouse tracking (all modes)
    /// CSI ?1003h CSI ?1015h CSI ?1006h
    #[must_use]
    pub fn enable_mouse_tracking() -> String {
        let mut result = String::new();
        // SGR Mouse Mode (1006) - modern extended mode supporting mouse wheel and
        // movement
        result.push_str(
            &CsiSequence::EnablePrivateMode(PrivateModeType::Other(SGR_MOUSE_MODE))
                .to_string(),
        );
        // Application Mouse Tracking (1003) - motion reporting
        result.push_str(
            &CsiSequence::EnablePrivateMode(PrivateModeType::Other(
                APPLICATION_MOUSE_TRACKING,
            ))
            .to_string(),
        );
        // Mouse Mode Extension (1015) - URXVT mouse extension
        result.push_str(
            &CsiSequence::EnablePrivateMode(PrivateModeType::Other(
                URXVT_MOUSE_EXTENSION,
            ))
            .to_string(),
        );
        result
    }

    /// Disable mouse tracking
    /// CSI ?1003l CSI ?1015l CSI ?1006l
    #[must_use]
    pub fn disable_mouse_tracking() -> String {
        let mut result = String::new();
        // SGR Mouse Mode (1006)
        result.push_str(
            &CsiSequence::DisablePrivateMode(PrivateModeType::Other(SGR_MOUSE_MODE))
                .to_string(),
        );
        // Application Mouse Tracking (1003)
        result.push_str(
            &CsiSequence::DisablePrivateMode(PrivateModeType::Other(
                APPLICATION_MOUSE_TRACKING,
            ))
            .to_string(),
        );
        // Mouse Mode Extension (1015)
        result.push_str(
            &CsiSequence::DisablePrivateMode(PrivateModeType::Other(
                URXVT_MOUSE_EXTENSION,
            ))
            .to_string(),
        );
        result
    }

    /// Enable bracketed paste mode
    /// CSI ?2004h
    #[must_use]
    pub fn enable_bracketed_paste() -> String {
        CsiSequence::EnablePrivateMode(PrivateModeType::Other(BRACKETED_PASTE_MODE))
            .to_string()
    }

    /// Disable bracketed paste mode
    /// CSI ?2004l
    #[must_use]
    pub fn disable_bracketed_paste() -> String {
        CsiSequence::DisablePrivateMode(PrivateModeType::Other(BRACKETED_PASTE_MODE))
            .to_string()
    }
}
