// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Margin setting operations (DECSTBM).

use std::cmp::min;

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MarginRequest};

/// Handle Set Top and Bottom Margins (DECSTBM) command.
/// CSI r - ESC [ top ; bottom r
///
/// This command sets the scrolling region for the terminal. Lines outside
/// the scrolling region are not affected by scroll operations.
pub fn set_margins(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let request = MarginRequest::from(params);
    let buffer_height: u16 = performer.ofs_buf.window_size.row_height.into();

    match request {
        MarginRequest::Reset => {
            // Reset scroll region to full screen.
            performer.ofs_buf.ansi_parser_support.scroll_region_top = None;
            performer.ofs_buf.ansi_parser_support.scroll_region_bottom = None;
        }
        MarginRequest::SetRegion { top, bottom } => {
            let top_value = top.as_u16();
            let bottom_value = bottom.as_u16();

            // Validate margins against buffer height.
            let clamped_bottom = min(bottom_value, buffer_height);

            if top_value < clamped_bottom && clamped_bottom <= buffer_height {
                performer.ofs_buf.ansi_parser_support.scroll_region_top = Some(top);
                performer.ofs_buf.ansi_parser_support.scroll_region_bottom =
                    Some(super::super::term_units::term_row(clamped_bottom));
            } else {
                tracing::warn!(
                    "CSI r (DECSTBM): Invalid margins top={}, bottom={}, buffer_height={}",
                    top_value,
                    clamped_bottom,
                    buffer_height
                );
            }
        }
    }
}
