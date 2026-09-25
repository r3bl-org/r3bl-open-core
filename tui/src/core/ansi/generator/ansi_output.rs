// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words URXVT

//! Generates [`ANSI`] escape sequence strings for terminal operations.
//!
//! This module generates raw [`ANSI`] escape sequence bytes for terminal operations using
//! semantic types and traits for type-safe sequence generation. Works in conjunction with
//! [`vt_100_pty_output_parser`] for bidirectional [`ANSI`] handling.
//!
//! # Design Philosophy
//!
//! - **No raw format! calls**: All sequences are generated using typed enums +
//!   [`FastStringify`]
//! - **Type-safe sequences**: Uses [`CsiSequence`], [`SgrColorSequence`], and other
//!   sequence enums
//! - **Reuses infrastructure**: Leverages existing [`ANSI`] types and constants
//! - **Infallible generation**: Exhaustive pattern matching ensures valid output
//! - **1-based indexing**: Automatically converts 0-based indices to 1-based [`ANSI`]
//!
//! # Reference Implementation Pattern
//!
//! Instead of raw format strings (❌ DON'T DO THIS):
//! ```rust
//! # use r3bl_tui::{vp_row, vp_col, CSI_START};
//! let row_idx = vp_row(5);
//! let col_idx = vp_col(10);
//! // Avoid this approach:
//! let s = format!("{CSI_START}{};{}H", row_idx.as_usize() + 1, col_idx.as_usize() + 1);
//! ```
//!
//! We now use semantic enums (✅ CURRENT APPROACH):
//! ```rust
//! # use r3bl_tui::{vp_row, vp_col, ansi_output};
//! let row_idx = vp_row(5);
//! let col_idx = vp_col(10);
//! let s = r3bl_tui::ansi_output::cursor_movement::cursor_position(row_idx.into(), col_idx.into());
//! assert_eq!(s, "\x1b[6;11H");  // row 5 → 6, col 10 → 11 (1-based)
//! ```
//!
//! This achieves:
//! - **Semantic clarity**: Intent is explicit (method name shows what we're doing)
//! - **Type safety**: Only valid sequences can be constructed via enums
//! - **Infallibility**: `FastStringify` guarantees valid [`ANSI`]
//! - **Consistency**: Matches test infrastructure patterns
//! - **No allocation waste**: Returns String, avoiding [`String`]→[`Vec<u8>`] conversion
//!
//! # Return Types
//!
//! All methods return [`String`] containing raw [`ANSI`] escape sequence bytes. Callers
//! can:
//! - Use directly if a [`String`] is needed
//! - Call [`.into_bytes()`] to get [`Vec<u8>`] for writing to output devices
//! - Use `&sequence` to get [`&str`] for slice operations
//!
//! This module has no state; it's a collection of static methods grouped by sub-modules.
//! State tracking (cursor position, current colors) is handled by external
//! implementations.
//!
//! [`.into_bytes()`]: String::into_bytes()
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CsiSequence`]: crate::CsiSequence
//! [`FastStringify`]: crate::fast_stringify::FastStringify
//! [`SgrColorSequence`]: crate::SgrColorSequence
//! [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
use crate::{ColorTarget, SgrColorSequence, TermRowDelta, TuiColor, TuiStyle,
            core::{ansi::{constants::{ALT_SCREEN_DISABLE_STR, ALT_SCREEN_ENABLE_STR,
                                      ANSI_CSI_U_CHAR, APPLICATION_MOUSE_TRACKING,
                                      BRACKETED_PASTE_MODE, CSI_ERASE_DISPLAY_ALL,
                                      CSI_ERASE_DISPLAY_TO_END, CSI_PARAM_SEPARATOR,
                                      CSI_PRIVATE_MODE_PREFIX, CSI_START,
                                      DECAWM_AUTO_WRAP, DECTCEM_SHOW_CURSOR,
                                      EL_ERASE_ALL, EL_ERASE_FROM_START,
                                      EL_ERASE_LINE, EL_ERASE_TO_END,
                                      KITTY_DISAMBIGUATE_ESCAPE_CODES,
                                      KITTY_POP_KEYBOARD_FLAGS,
                                      KITTY_PUSH_KEYBOARD_FLAGS,
                                      RCP_RESTORE_CURSOR_STR, RM_RESET_MODE,
                                      SCP_SAVE_CURSOR_STR, SGR_BOLD, SGR_DIM,
                                      SGR_ITALIC, SGR_MOUSE_MODE, SGR_RESET_STR,
                                      SGR_SET_GRAPHICS, SGR_STRIKETHROUGH,
                                      SGR_UNDERLINE, SM_SET_MODE,
                                      URXVT_MOUSE_EXTENSION},
                          vt_100_pty_output_parser::CsiSequence},
                   coordinates::{TermCol, TermRow}}};

pub mod cursor_movement {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Generate absolute cursor positioning
    /// [`CSI`] `<row>;<col>H` (1-based indexing)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn cursor_position(row: TermRow, col: TermCol) -> String {
        CsiSequence::CursorPosition { row, col }.to_string()
    }

    /// Generate cursor to column
    /// [`CSI`] <col>G (1-based)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn cursor_to_column(col: TermCol) -> String {
        CsiSequence::CursorHorizontalAbsolute(col).to_string()
    }

    /// Generate cursor next line.
    ///
    /// [`CSI`] `<n>E`
    ///
    /// Uses [`TermRowDelta`] for type-safe cursor movement. Since [`TermRowDelta`]
    /// wraps [`NonZeroU16`] internally, the [`CSI`] zero bug is prevented at the type
    /// level—callers use [`TermRowDelta::new()`] which returns `None` for zero.
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`NonZeroU16`]: std::num::NonZeroU16
    #[must_use]
    pub fn cursor_next_line(rows: TermRowDelta) -> String {
        CsiSequence::CursorNextLine(rows).to_string()
    }

    /// Generate cursor previous line.
    ///
    /// [`CSI`] `<n>F`
    ///
    /// Uses [`TermRowDelta`] for type-safe cursor movement. Since [`TermRowDelta`]
    /// wraps [`NonZeroU16`] internally, the [`CSI`] zero bug is prevented at the type
    /// level—callers use [`TermRowDelta::new()`] which returns `None` for zero.
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`NonZeroU16`]: std::num::NonZeroU16
    #[must_use]
    pub fn cursor_previous_line(rows: TermRowDelta) -> String {
        CsiSequence::CursorPrevLine(rows).to_string()
    }
}

pub mod screen_clearing {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Clear entire screen
    /// [`CSI`] 2J (Erase Display: 2 = entire display)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn clear_screen() -> &'static str { CSI_ERASE_DISPLAY_ALL }

    /// Clear to end of screen
    /// [`CSI`] 0J (Erase Display: 0 = cursor to end)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn clear_to_end_of_screen() -> &'static str { CSI_ERASE_DISPLAY_TO_END }

    /// Clear current line
    /// [`CSI`] 2K (Erase Line: 2 = entire line)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn clear_current_line() -> &'static str {
        const_format::concatcp!(CSI_START, EL_ERASE_ALL, EL_ERASE_LINE)
    }

    /// Clear to end of line
    /// [`CSI`] 0K (Erase Line: 0 = cursor to end)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn clear_to_end_of_line() -> &'static str {
        const_format::concatcp!(CSI_START, EL_ERASE_TO_END, EL_ERASE_LINE)
    }

    /// Clear to start of line
    /// [`CSI`] 1K (Erase Line: 1 = start to cursor)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn clear_to_start_of_line() -> &'static str {
        const_format::concatcp!(CSI_START, EL_ERASE_FROM_START, EL_ERASE_LINE)
    }
}

pub mod color_ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
    /// Uses semantic [`SGR`] codes from the [`vt_100_pty_output_parser`]
    /// infrastructure
    ///
    /// [`SGR`]: crate::SgrCode
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
    /// [`CSI`] 0m ([`SGR`] Reset)
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`SGR`]: crate::SgrCode
    #[must_use]
    pub fn reset_color() -> &'static str { SGR_RESET_STR }
}

pub mod cursor_visibility {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Show cursor
    /// [`CSI`] ?25h (DECTCEM: [`DEC`] Text Cursor Enable Mode = set)
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    #[must_use]
    pub fn show_cursor() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            DECTCEM_SHOW_CURSOR,
            SM_SET_MODE,
        )
    }

    /// Hide cursor
    /// [`CSI`] ?25l (DECTCEM = reset)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn hide_cursor() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            DECTCEM_SHOW_CURSOR,
            RM_RESET_MODE,
        )
    }
}

pub mod cursor_save_restore {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Save cursor position - [`CSI`] s (Save Cursor, i.e., [`DECSC`]).
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`DECSC`]: https://vt100.net/docs/vt510-rm/DECSC.html
    #[must_use]
    pub fn save_cursor_position() -> &'static str { SCP_SAVE_CURSOR_STR }

    /// Restore cursor position - [`CSI`] u (Restore Cursor, i.e., [`DECRC`]).
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`DECRC`]: https://vt100.net/docs/vt510-rm/DECRC.html
    #[must_use]
    pub fn restore_cursor_position() -> &'static str { RCP_RESTORE_CURSOR_STR }
}

pub mod terminal_modes {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Enter alternate screen buffer
    /// [`CSI`] ?1049h (`AlternateScreenBuffer`)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn enter_alternate_screen() -> &'static str { ALT_SCREEN_ENABLE_STR }

    /// Exit alternate screen buffer
    /// [`CSI`] ?1049l (`AlternateScreenBuffer`)
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn exit_alternate_screen() -> &'static str { ALT_SCREEN_DISABLE_STR }

    /// Enable mouse tracking (all modes)
    /// [`CSI`] ?1003h [`CSI`] ?1015h [`CSI`] ?1006h
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn enable_mouse_tracking() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            APPLICATION_MOUSE_TRACKING,
            CSI_PARAM_SEPARATOR,
            URXVT_MOUSE_EXTENSION,
            CSI_PARAM_SEPARATOR,
            SGR_MOUSE_MODE,
            SM_SET_MODE,
        )
    }

    /// Disable mouse tracking
    /// [`CSI`] ?1006l [`CSI`] ?1015l [`CSI`] ?1003l
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn disable_mouse_tracking() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            SGR_MOUSE_MODE,
            CSI_PARAM_SEPARATOR,
            URXVT_MOUSE_EXTENSION,
            CSI_PARAM_SEPARATOR,
            APPLICATION_MOUSE_TRACKING,
            RM_RESET_MODE,
        )
    }

    /// Enable bracketed paste mode
    /// [`CSI`] `?2004h`
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn enable_bracketed_paste() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            BRACKETED_PASTE_MODE,
            SM_SET_MODE,
        )
    }

    /// Disable bracketed paste mode
    /// [`CSI`] `?2004l`
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn disable_bracketed_paste() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            BRACKETED_PASTE_MODE,
            RM_RESET_MODE,
        )
    }

    /// Enable progressive keyboard enhancement ([`Kitty`] keyboard protocol).
    /// [`CSI`] `>1u` (Push flags: 1 = `DISAMBIGUATE_ESCAPE_CODES`).
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
    #[must_use]
    pub fn enable_keyboard_enhancement() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            KITTY_PUSH_KEYBOARD_FLAGS,
            KITTY_DISAMBIGUATE_ESCAPE_CODES,
            ANSI_CSI_U_CHAR,
        )
    }

    /// Disable progressive keyboard enhancement ([`Kitty`] keyboard protocol).
    /// [`CSI`] `<1u` (Pop 1 level of flags from the stack).
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
    #[must_use]
    pub fn disable_keyboard_enhancement() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            KITTY_POP_KEYBOARD_FLAGS,
            KITTY_DISAMBIGUATE_ESCAPE_CODES,
            ANSI_CSI_U_CHAR,
        )
    }

    /// Enable line wrap ([`DECAWM`]: [`DEC`] Autowrap Mode = set).
    /// [`CSI`] `?7h`
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    #[must_use]
    pub fn enable_line_wrap() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            DECAWM_AUTO_WRAP,
            SM_SET_MODE,
        )
    }

    /// Disable line wrap ([`DECAWM`]: [`DEC`] Autowrap Mode = reset).
    /// [`CSI`] `?7l`
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    #[must_use]
    pub fn disable_line_wrap() -> &'static str {
        const_format::concatcp!(
            CSI_START,
            CSI_PRIVATE_MODE_PREFIX,
            DECAWM_AUTO_WRAP,
            RM_RESET_MODE,
        )
    }
}
