// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal state operations.

use super::super::super::ansi_parser_public_api::AnsiToBufferProcessor;
use crate::{CharacterSet, PixelChar, Pos};

/// Clear all buffer content.
fn clear_buffer(processor: &mut AnsiToBufferProcessor) {
    let max_row = processor.ofs_buf.window_size.row_height.as_usize();
    for row in 0..max_row {
        for col in 0..processor.ofs_buf.window_size.col_width.as_usize() {
            processor.ofs_buf.buffer[row][col] = PixelChar::Spacer;
        }
    }
}

/// Reset all SGR attributes to default state.
fn reset_sgr_attributes(processor: &mut AnsiToBufferProcessor) {
    processor.ofs_buf.ansi_parser_support.current_style = None;
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

/// Reset terminal to initial state (ESC c).
/// Clears the buffer, resets cursor, and clears saved state.
/// Clears DECSTBM scroll region margins.
pub fn reset_terminal(processor: &mut AnsiToBufferProcessor) {
    clear_buffer(processor);

    // Reset cursor to home position
    processor.ofs_buf.my_pos = Pos::default();

    // Clear saved cursor state
    processor
        .ofs_buf
        .ansi_parser_support
        .cursor_pos_for_esc_save_and_restore = None;

    // Reset to ASCII character set
    processor.ofs_buf.ansi_parser_support.character_set = CharacterSet::Ascii;

    // Clear DECSTBM scroll region margins
    processor.ofs_buf.ansi_parser_support.scroll_region_top = None;
    processor.ofs_buf.ansi_parser_support.scroll_region_bottom = None;

    // Clear any SGR attributes
    reset_sgr_attributes(processor);

    tracing::trace!("ESC c: Terminal reset to initial state");
}