// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Style/Graphics Rendition operations.

use vte::Params;

use super::super::super::{ansi_parser_public_api::AnsiToBufferProcessor, 
                        ansi_to_tui_color::ansi_to_tui_color,
                        csi_codes};
use crate::{TuiStyle, tui_style_attrib};

/// Update the current `TuiStyle` based on SGR attributes.
pub fn update_style(processor: &mut AnsiToBufferProcessor) {
    processor.ofs_buf.ansi_parser_support.current_style = Some(TuiStyle {
        id: None,
        attribs: processor.ofs_buf.ansi_parser_support.attribs,
        computed: None,
        color_fg: processor.ofs_buf.ansi_parser_support.fg_color,
        color_bg: processor.ofs_buf.ansi_parser_support.bg_color,
        padding: None,
        lolcat: None,
    });
}

/// Reset all SGR attributes to default state.
fn reset_all_attributes(processor: &mut AnsiToBufferProcessor) {
    processor.ofs_buf.ansi_parser_support.attribs.bold = None;
    processor.ofs_buf.ansi_parser_support.attribs.dim = None;
    processor.ofs_buf.ansi_parser_support.attribs.italic = None;
    processor.ofs_buf.ansi_parser_support.attribs.underline = None;
    processor.ofs_buf.ansi_parser_support.attribs.blink = None;
    processor.ofs_buf.ansi_parser_support.attribs.reverse = None;
    processor.ofs_buf.ansi_parser_support.attribs.hidden = None;
    processor.ofs_buf.ansi_parser_support.attribs.strikethrough = None;
    processor.ofs_buf.ansi_parser_support.fg_color = None;
    processor.ofs_buf.ansi_parser_support.bg_color = None;
}

/// Apply a single SGR parameter.
fn apply_sgr_param(processor: &mut AnsiToBufferProcessor, param: u16) {
    match param {
        csi_codes::SGR_RESET => {
            reset_all_attributes(processor);
        }
        csi_codes::SGR_BOLD => {
            processor.ofs_buf.ansi_parser_support.attribs.bold =
                Some(tui_style_attrib::Bold)
        }
        csi_codes::SGR_DIM => {
            processor.ofs_buf.ansi_parser_support.attribs.dim =
                Some(tui_style_attrib::Dim)
        }
        csi_codes::SGR_ITALIC => {
            processor.ofs_buf.ansi_parser_support.attribs.italic =
                Some(tui_style_attrib::Italic);
        }
        csi_codes::SGR_UNDERLINE => {
            processor.ofs_buf.ansi_parser_support.attribs.underline =
                Some(tui_style_attrib::Underline);
        }
        csi_codes::SGR_BLINK | csi_codes::SGR_RAPID_BLINK => {
            processor.ofs_buf.ansi_parser_support.attribs.blink =
                Some(tui_style_attrib::Blink);
        }
        csi_codes::SGR_REVERSE => {
            processor.ofs_buf.ansi_parser_support.attribs.reverse =
                Some(tui_style_attrib::Reverse);
        }
        csi_codes::SGR_HIDDEN => {
            processor.ofs_buf.ansi_parser_support.attribs.hidden =
                Some(tui_style_attrib::Hidden);
        }
        csi_codes::SGR_STRIKETHROUGH => {
            processor.ofs_buf.ansi_parser_support.attribs.strikethrough =
                Some(tui_style_attrib::Strikethrough);
        }
        csi_codes::SGR_RESET_BOLD_DIM => {
            processor.ofs_buf.ansi_parser_support.attribs.bold = None;
            processor.ofs_buf.ansi_parser_support.attribs.dim = None;
        }
        csi_codes::SGR_RESET_ITALIC => {
            processor.ofs_buf.ansi_parser_support.attribs.italic = None
        }
        csi_codes::SGR_RESET_UNDERLINE => {
            processor.ofs_buf.ansi_parser_support.attribs.underline = None
        }
        csi_codes::SGR_RESET_BLINK => {
            processor.ofs_buf.ansi_parser_support.attribs.blink = None
        }
        csi_codes::SGR_RESET_REVERSE => {
            processor.ofs_buf.ansi_parser_support.attribs.reverse = None
        }
        csi_codes::SGR_RESET_HIDDEN => {
            processor.ofs_buf.ansi_parser_support.attribs.hidden = None
        }
        csi_codes::SGR_RESET_STRIKETHROUGH => {
            processor.ofs_buf.ansi_parser_support.attribs.strikethrough = None
        }
        csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
            processor.ofs_buf.ansi_parser_support.fg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        csi_codes::SGR_FG_DEFAULT => {
            processor.ofs_buf.ansi_parser_support.fg_color = None
        } /* Default foreground */
        csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
            processor.ofs_buf.ansi_parser_support.bg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        csi_codes::SGR_BG_DEFAULT => {
            processor.ofs_buf.ansi_parser_support.bg_color = None
        } /* Default background */
        csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
            processor.ofs_buf.ansi_parser_support.fg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
            processor.ofs_buf.ansi_parser_support.bg_color =
                Some(ansi_to_tui_color(param.into()));
        }
        _ => {} /* Ignore unsupported SGR parameters (256-color, RGB, etc.) */
    }
}

/// Handle SGR (Select Graphic Rendition) parameters.
pub fn sgr(processor: &mut AnsiToBufferProcessor, params: &Params) {
    for param_slice in params {
        for &param in param_slice {
            apply_sgr_param(processor, param);
        }
    }
    update_style(processor);
}