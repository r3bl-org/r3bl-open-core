// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Style/Graphics Rendition operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the operations module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_sgr_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_sgr_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in the conformance tests validating the full pipeline
//!
//! For the complete testing philosophy,
//! and rationale behind this approach.
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭────────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │────▶ PTY Controller │────▶ VTE Parser      │────▶ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream)  │    │ (state machine) │    │ (terminal    │
//! ╰──────┬──────────╯    ╰────────────────╯    ╰───────┬─────────╯    │  buffer)     │
//!        │                                             │              ╰───────┬──────╯
//!        │                                             │                      │
//!        │                                    ╔════════▼════════╗             │
//!        │                                    ║ Perform Trait   ║             │
//!        │                                    ║ Implementation  ║             │
//!        │                                    ╚═════════════════╝             │
//!        │                                                                    │
//!        │                                    ╭─────────────────╮             │
//!        │                                    │ RenderPipeline  ◀─────────────╯
//!        │                                    │ paint()         │
//!        ╰────────────────────────────────────▶ Terminal Output │
//!                                             ╰─────────────────╯
//! ```
//!
//! # `CSI` Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC [1;31m" (bold red text)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses `ESC [`...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T) ╭───────────╮
//!       - sgr_ops:: for styling (m) <----- │THIS MODULE│
//!       - line_ops:: for lines (L,M)       ╰───────────╯
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! [`impl_sgr_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_sgr_ops
//! [`test_sgr_ops`]: crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::tests::vt_100_test_sgr_ops
//! [module-level documentation]: self

use super::super::{AnsiToOfsBufPerformer, ParamsExt};
use crate::{SgrColorSequence,
            core::ansi::constants::{SGR_BG_BLACK, SGR_BG_BRIGHT_BLACK,
                                    SGR_BG_BRIGHT_WHITE, SGR_BG_DEFAULT,
                                    SGR_BG_EXTENDED, SGR_BG_WHITE, SGR_BLINK, SGR_BOLD,
                                    SGR_COLOR_MODE_256, SGR_COLOR_MODE_RGB, SGR_DIM,
                                    SGR_FG_BLACK, SGR_FG_BRIGHT_BLACK,
                                    SGR_FG_BRIGHT_WHITE, SGR_FG_DEFAULT,
                                    SGR_FG_EXTENDED, SGR_FG_WHITE, SGR_HIDDEN,
                                    SGR_ITALIC, SGR_RAPID_BLINK, SGR_RESET,
                                    SGR_RESET_BLINK, SGR_RESET_BOLD_DIM,
                                    SGR_RESET_HIDDEN, SGR_RESET_ITALIC,
                                    SGR_RESET_REVERSE, SGR_RESET_STRIKETHROUGH,
                                    SGR_RESET_UNDERLINE, SGR_REVERSE,
                                    SGR_STRIKETHROUGH, SGR_UNDERLINE},
            tui_style_attrib};
use vte::Params;

/// Apply a single `SGR` parameter.
#[allow(clippy::too_many_lines)]
fn apply_sgr_param(performer: &mut AnsiToOfsBufPerformer, param: u16) {
    match param {
        SGR_RESET => {
            performer.ofs_buf.reset_all_style_attributes();
        }
        SGR_BOLD => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Bold.into());
        }
        SGR_DIM => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Dim.into());
        }
        SGR_ITALIC => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Italic.into());
        }
        SGR_UNDERLINE => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Underline.into());
        }
        SGR_BLINK => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::BlinkMode::Slow.into());
        }
        SGR_RAPID_BLINK => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::BlinkMode::Rapid.into());
        }
        SGR_REVERSE => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Reverse.into());
        }
        SGR_HIDDEN => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Hidden.into());
        }
        SGR_STRIKETHROUGH => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Strikethrough.into());
        }
        SGR_RESET_BOLD_DIM => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Bold.into());
        }
        SGR_RESET_ITALIC => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Italic.into());
        }
        SGR_RESET_UNDERLINE => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Underline.into());
        }
        SGR_RESET_BLINK => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::BlinkMode::Slow.into());
        }
        SGR_RESET_REVERSE => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Reverse.into());
        }
        SGR_RESET_HIDDEN => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Hidden.into());
        }
        SGR_RESET_STRIKETHROUGH => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Strikethrough.into());
        }
        SGR_FG_BLACK..=SGR_FG_WHITE => {
            performer.ofs_buf.set_foreground_color(param);
        }
        SGR_FG_DEFAULT => {
            performer.ofs_buf.reset_foreground_color();
        }
        SGR_BG_BLACK..=SGR_BG_WHITE => {
            performer.ofs_buf.set_background_color(param);
        }
        SGR_BG_DEFAULT => {
            performer.ofs_buf.reset_background_color();
        }
        SGR_FG_BRIGHT_BLACK..=SGR_FG_BRIGHT_WHITE => {
            performer.ofs_buf.set_foreground_color(param);
        }
        SGR_BG_BRIGHT_BLACK..=SGR_BG_BRIGHT_WHITE => {
            performer.ofs_buf.set_background_color(param);
        }
        _ => {
            // Ignore other unsupported SGR parameters:
            // - SGR 10-21, 26, 30-36, 38-47, 53-60, etc. (reserved/rarely-used)
            // - SGR 38/48 without 5/2 mode (malformed; extended colors handled upstream)
        }
    }
}

/// Handle `SGR` (Select Graphic Rendition) parameters.
///
/// This function processes `SGR` parameters and applies them to the offscreen buffer.
/// It supports:
/// - Basic text attributes (bold, italic, underline, etc.)
/// - 16-color ANSI colors (30-37, 40-47, 90-97, 100-107)
/// - 256-color palette (colon format `ESC[38:5:nm` or legacy `ESC[38;5;nm`)
/// - RGB true color (colon format `ESC[38:2:r:g:bm` or legacy `ESC[38;2;r;g;bm`)
///
/// # Extended Color Format Handling
///
/// Extended colors (256-color and RGB) can be formatted two ways:
///
/// - **Colon format** ([ITU-T Rec. T.416]): `ESC[38:5:196m` - VTE groups as `[[38, 5,
///   196]]`
/// - **Semicolon format** (legacy): `ESC[38;5;196m` - VTE parses as `[[38], [5], [196]]`
///
/// The colon format is handled directly by [`SgrColorSequence::parse_from_raw_slice`].
/// The semicolon format requires look-ahead logic to collect params from subsequent
/// positions.
///
/// [ITU-T Rec. T.416]: https://www.itu.int/rec/T-REC-T.416-199303-I
pub fn set_graphics_rendition(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let mut idx = 0;
    while let Some(param_slice) = params.extract_nth_many_raw(idx) {
        // Case 1: Colon-separated format (already grouped by VTE as a single slice).
        if let Some(color_seq) = SgrColorSequence::parse_from_raw_slice(param_slice) {
            performer.ofs_buf.apply_extended_color_sequence(color_seq);
            idx += 1;
            continue;
        }

        // Case 2: Check for semicolon-separated extended color (single-element slice).
        if let Some(&first_param) = param_slice.first() {
            if param_slice.len() == 1
                && (first_param == SGR_FG_EXTENDED || first_param == SGR_BG_EXTENDED)
            {
                // Try to collect semicolon-separated extended color params.
                if let Some(consumed) = try_parse_semicolon_extended_color(
                    performer,
                    params,
                    idx,
                    first_param,
                ) {
                    idx += consumed;
                    continue;
                }
            }
            // Case 3: Regular single SGR parameter.
            apply_sgr_param(performer, first_param);
        }
        idx += 1;
    }
}

/// Try to parse semicolon-separated extended color starting at `start_idx`.
///
/// When VTE encounters semicolon-separated extended colors like `ESC[38;5;196m`,
/// it produces separate parameter positions: `[[38], [5], [196]]`. This function
/// looks ahead to collect the remaining params and apply the color.
///
/// # Returns
///
/// - `Some(params_consumed)` - Successfully parsed; caller should skip this many params
/// - `None` - Not a valid extended color sequence; caller should handle normally
#[allow(clippy::cast_possible_truncation)] // Values are validated <= 255 before cast.
fn try_parse_semicolon_extended_color(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
    start_idx: usize,
    fg_or_bg: u16,
) -> Option<usize> {
    // Get mode from next position (5 = 256-color, 2 = RGB).
    let mode = params.extract_nth_single_opt_raw(start_idx + 1)?;

    match mode {
        SGR_COLOR_MODE_256 => {
            // 256-color: need 1 more param (color index).
            let index = params.extract_nth_single_opt_raw(start_idx + 2)?;
            if index > 255 {
                return None;
            }

            let color_seq = if fg_or_bg == SGR_FG_EXTENDED {
                SgrColorSequence::SetForegroundAnsi256(index as u8)
            } else {
                SgrColorSequence::SetBackgroundAnsi256(index as u8)
            };
            performer.ofs_buf.apply_extended_color_sequence(color_seq);
            Some(3) // Consumed: [38/48], [5], [index]
        }
        SGR_COLOR_MODE_RGB => {
            // RGB: need 3 more params (r, g, b).
            let r = params.extract_nth_single_opt_raw(start_idx + 2)?;
            let g = params.extract_nth_single_opt_raw(start_idx + 3)?;
            let b = params.extract_nth_single_opt_raw(start_idx + 4)?;
            if r > 255 || g > 255 || b > 255 {
                return None;
            }

            let color_seq = if fg_or_bg == SGR_FG_EXTENDED {
                SgrColorSequence::SetForegroundRgb(r as u8, g as u8, b as u8)
            } else {
                SgrColorSequence::SetBackgroundRgb(r as u8, g as u8, b as u8)
            };
            performer.ofs_buf.apply_extended_color_sequence(color_seq);
            Some(5) // Consumed: [38/48], [2], [r], [g], [b]
        }
        _ => None, // Unknown mode, not an extended color.
    }
}
