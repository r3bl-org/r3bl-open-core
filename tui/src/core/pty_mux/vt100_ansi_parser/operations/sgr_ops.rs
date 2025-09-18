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

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   ansi_to_tui_color::ansi_to_tui_color, protocols::csi_codes};
use crate::{TuiStyle, tui_style_attrib};

/// Macro to set a field on `current_style`.
macro_rules! set_style_field {
    ($performer:expr, $($field:tt).+ = $value:expr) => {
        $performer.ofs_buf.ansi_parser_support.current_style.$($field).+ = $value;
    };
}

/// Reset all SGR attributes to default state.
fn reset_all_attributes(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.ansi_parser_support.current_style = TuiStyle::default();
}

/// Apply a single SGR parameter.
fn apply_sgr_param(performer: &mut AnsiToOfsBufPerformer, param: u16) {
    match param {
        csi_codes::SGR_RESET => {
            reset_all_attributes(performer);
        }
        csi_codes::SGR_BOLD => {
            set_style_field!(performer, attribs.bold = Some(tui_style_attrib::Bold));
        }
        csi_codes::SGR_DIM => {
            set_style_field!(performer, attribs.dim = Some(tui_style_attrib::Dim));
        }
        csi_codes::SGR_ITALIC => {
            set_style_field!(performer, attribs.italic = Some(tui_style_attrib::Italic));
        }
        csi_codes::SGR_UNDERLINE => {
            set_style_field!(
                performer,
                attribs.underline = Some(tui_style_attrib::Underline)
            );
        }
        csi_codes::SGR_BLINK | csi_codes::SGR_RAPID_BLINK => {
            set_style_field!(performer, attribs.blink = Some(tui_style_attrib::Blink));
        }
        csi_codes::SGR_REVERSE => {
            set_style_field!(
                performer,
                attribs.reverse = Some(tui_style_attrib::Reverse)
            );
        }
        csi_codes::SGR_HIDDEN => {
            set_style_field!(performer, attribs.hidden = Some(tui_style_attrib::Hidden));
        }
        csi_codes::SGR_STRIKETHROUGH => {
            set_style_field!(
                performer,
                attribs.strikethrough = Some(tui_style_attrib::Strikethrough)
            );
        }
        csi_codes::SGR_RESET_BOLD_DIM => {
            let style = &mut performer.ofs_buf.ansi_parser_support.current_style;
            style.attribs.bold = None;
            style.attribs.dim = None;
        }
        csi_codes::SGR_RESET_ITALIC => {
            set_style_field!(performer, attribs.italic = None);
        }
        csi_codes::SGR_RESET_UNDERLINE => {
            set_style_field!(performer, attribs.underline = None);
        }
        csi_codes::SGR_RESET_BLINK => {
            set_style_field!(performer, attribs.blink = None);
        }
        csi_codes::SGR_RESET_REVERSE => {
            set_style_field!(performer, attribs.reverse = None);
        }
        csi_codes::SGR_RESET_HIDDEN => {
            set_style_field!(performer, attribs.hidden = None);
        }
        csi_codes::SGR_RESET_STRIKETHROUGH => {
            set_style_field!(performer, attribs.strikethrough = None);
        }
        csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
            set_style_field!(performer, color_fg = Some(ansi_to_tui_color(param.into())));
        }
        csi_codes::SGR_FG_DEFAULT => {
            set_style_field!(performer, color_fg = None);
        } /* Default foreground */
        csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
            set_style_field!(performer, color_bg = Some(ansi_to_tui_color(param.into())));
        }
        csi_codes::SGR_BG_DEFAULT => {
            set_style_field!(performer, color_bg = None);
        } /* Default background */
        csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
            set_style_field!(performer, color_fg = Some(ansi_to_tui_color(param.into())));
        }
        csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
            set_style_field!(performer, color_bg = Some(ansi_to_tui_color(param.into())));
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
