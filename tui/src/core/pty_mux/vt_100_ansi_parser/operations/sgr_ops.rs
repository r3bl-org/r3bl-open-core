// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Style/Graphics Rendition operations.
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭──────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master   │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream)│    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰──────────────╯    ╰─────────────────╯    │  buffer)     │
//!                                                     │             ╰──────────────╯
//!                                                     ▼
//!                                            ╭─────────────────╮
//!                                            │ Perform Trait   │
//!                                            │ Implementation  │
//!                                            ╰─────────────────╯
//! ```
//!
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[1;31m" (bold red text)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
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

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer, protocols::csi_codes};
// Import the StyleAttribute enum from the implementation module.
// This will be available once we update the mod.rs file.
use crate::tui::terminal_lib_backends::offscreen_buffer::vt100_ansi_impl::sgr_ops::StyleAttribute;

/// Reset all SGR attributes to default state.
fn reset_all_attributes(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.reset_all_style_attributes();
}

/// Apply a single SGR parameter.
fn apply_sgr_param(performer: &mut AnsiToOfsBufPerformer, param: u16) {
    match param {
        csi_codes::SGR_RESET => {
            reset_all_attributes(performer);
        }
        csi_codes::SGR_BOLD => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Bold);
        }
        csi_codes::SGR_DIM => {
            performer.ofs_buf.apply_style_attribute(StyleAttribute::Dim);
        }
        csi_codes::SGR_ITALIC => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Italic);
        }
        csi_codes::SGR_UNDERLINE => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Underline);
        }
        csi_codes::SGR_BLINK | csi_codes::SGR_RAPID_BLINK => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Blink);
        }
        csi_codes::SGR_REVERSE => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Reverse);
        }
        csi_codes::SGR_HIDDEN => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Hidden);
        }
        csi_codes::SGR_STRIKETHROUGH => {
            performer
                .ofs_buf
                .apply_style_attribute(StyleAttribute::Strikethrough);
        }
        csi_codes::SGR_RESET_BOLD_DIM => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Bold);
        }
        csi_codes::SGR_RESET_ITALIC => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Italic);
        }
        csi_codes::SGR_RESET_UNDERLINE => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Underline);
        }
        csi_codes::SGR_RESET_BLINK => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Blink);
        }
        csi_codes::SGR_RESET_REVERSE => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Reverse);
        }
        csi_codes::SGR_RESET_HIDDEN => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Hidden);
        }
        csi_codes::SGR_RESET_STRIKETHROUGH => {
            performer
                .ofs_buf
                .reset_style_attribute(StyleAttribute::Strikethrough);
        }
        csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
            performer.ofs_buf.set_foreground_color(param);
        }
        csi_codes::SGR_FG_DEFAULT => {
            performer.ofs_buf.reset_foreground_color();
        }
        csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
            performer.ofs_buf.set_background_color(param);
        }
        csi_codes::SGR_BG_DEFAULT => {
            performer.ofs_buf.reset_background_color();
        }
        csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
            performer.ofs_buf.set_foreground_color(param);
        }
        csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
            performer.ofs_buf.set_background_color(param);
        }
        _ => {} /* Ignore unsupported SGR parameters (256-color, RGB, etc.) */
    }
}

/// Handle SGR (Select Graphic Rendition) parameters.
pub fn set_graphics_rendition(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    for param_slice in params {
        for &param in param_slice {
            apply_sgr_param(performer, param);
        }
    }
}
