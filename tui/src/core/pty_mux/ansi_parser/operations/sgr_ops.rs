// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Style/Graphics Rendition operations.
//!
//! # CSI Sequence Architecture
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
//!     csi_dispatch() [THIS METHOD]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   ansi_to_tui_color::ansi_to_tui_color, protocols::csi_codes};
use crate::{TuiStyle, tui_style_attrib};

/// Update the current `TuiStyle` based on SGR attributes.
pub fn update_style(performer: &mut AnsiToOfsBufPerformer) {
    let attribs = performer.ofs_buf.ansi_parser_support.attribs;
    let fg_color = performer.ofs_buf.ansi_parser_support.fg_color;
    let bg_color = performer.ofs_buf.ansi_parser_support.bg_color;

    // If all attributes are None (after SGR reset), set current_style to None
    // This ensures plain text has no styling
    if attribs.is_none() && fg_color.is_none() && bg_color.is_none() {
        performer.ofs_buf.ansi_parser_support.current_style = None;
    } else {
        performer.ofs_buf.ansi_parser_support.current_style = Some(TuiStyle {
            id: None,
            attribs,
            computed: None,
            color_fg: fg_color,
            color_bg: bg_color,
            padding: None,
            lolcat: None,
        });
    }
}

/// Reset all SGR attributes to default state.
fn reset_all_attributes(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.ansi_parser_support.attribs.reset();
    performer.ofs_buf.ansi_parser_support.fg_color = None;
    performer.ofs_buf.ansi_parser_support.bg_color = None;
}

/// Apply a single SGR parameter.
fn apply_sgr_param(performer: &mut AnsiToOfsBufPerformer, param: u16) {
    match param {
        csi_codes::SGR_RESET => {
            reset_all_attributes(performer);
        }
        csi_codes::SGR_BOLD => {
            performer.ofs_buf.ansi_parser_support.attribs.bold =
                Some(tui_style_attrib::Bold);
        }
        csi_codes::SGR_DIM => {
            performer.ofs_buf.ansi_parser_support.attribs.dim =
                Some(tui_style_attrib::Dim);
        }
        csi_codes::SGR_ITALIC => {
            performer.ofs_buf.ansi_parser_support.attribs.italic =
                Some(tui_style_attrib::Italic);
        }
        csi_codes::SGR_UNDERLINE => {
            performer.ofs_buf.ansi_parser_support.attribs.underline =
                Some(tui_style_attrib::Underline);
        }
        csi_codes::SGR_BLINK | csi_codes::SGR_RAPID_BLINK => {
            performer.ofs_buf.ansi_parser_support.attribs.blink =
                Some(tui_style_attrib::Blink);
        }
        csi_codes::SGR_REVERSE => {
            performer.ofs_buf.ansi_parser_support.attribs.reverse =
                Some(tui_style_attrib::Reverse);
        }
        csi_codes::SGR_HIDDEN => {
            performer.ofs_buf.ansi_parser_support.attribs.hidden =
                Some(tui_style_attrib::Hidden);
        }
        csi_codes::SGR_STRIKETHROUGH => {
            performer.ofs_buf.ansi_parser_support.attribs.strikethrough =
                Some(tui_style_attrib::Strikethrough);
        }
        csi_codes::SGR_RESET_BOLD_DIM => {
            performer.ofs_buf.ansi_parser_support.attribs.bold = None;
            performer.ofs_buf.ansi_parser_support.attribs.dim = None;
        }
        csi_codes::SGR_RESET_ITALIC => {
            performer.ofs_buf.ansi_parser_support.attribs.italic = None;
        }
        csi_codes::SGR_RESET_UNDERLINE => {
            performer.ofs_buf.ansi_parser_support.attribs.underline = None;
        }
        csi_codes::SGR_RESET_BLINK => {
            performer.ofs_buf.ansi_parser_support.attribs.blink = None;
        }
        csi_codes::SGR_RESET_REVERSE => {
            performer.ofs_buf.ansi_parser_support.attribs.reverse = None;
        }
        csi_codes::SGR_RESET_HIDDEN => {
            performer.ofs_buf.ansi_parser_support.attribs.hidden = None;
        }
        csi_codes::SGR_RESET_STRIKETHROUGH => {
            performer.ofs_buf.ansi_parser_support.attribs.strikethrough = None;
        }
        csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
            performer.ofs_buf.ansi_parser_support.fg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        csi_codes::SGR_FG_DEFAULT => {
            performer.ofs_buf.ansi_parser_support.fg_color = None;
        } /* Default foreground */
        csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
            performer.ofs_buf.ansi_parser_support.bg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        csi_codes::SGR_BG_DEFAULT => {
            performer.ofs_buf.ansi_parser_support.bg_color = None;
        } /* Default background */
        csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
            performer.ofs_buf.ansi_parser_support.fg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
            performer.ofs_buf.ansi_parser_support.bg_color =
                Some(ansi_to_tui_color(param.into()));
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
    update_style(performer);
}
