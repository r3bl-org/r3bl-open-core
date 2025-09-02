// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Margin setting operations (DECSTBM).

use vte::Params;

use crate::ansi_parser_perform_impl::param_utils::ParamsExt;

use super::super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                          term_units::term_row};

/// Handle Set Top and Bottom Margins (DECSTBM) command.
/// CSI r - ESC [ top ; bottom r
///
/// This command sets the scrolling region for the terminal. Lines outside
/// the scrolling region are not affected by scroll operations.
pub fn set_margins(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let maybe_top = params.extract_nth_opt(0);
    let maybe_bottom = params.extract_nth_opt(1);

    // Store terminal's 1-based coordinates (will be converted to 0-based when used).
    let buffer_height: u16 = processor.ofs_buf.window_size.row_height.into();

    match (maybe_top, maybe_bottom) {
        // Reset scroll region to full screen.
        // Handles: ESC [ r (no params), ESC [ 0 r, ESC [ 0 ; 0 r
        // All these cases mean "reset to use the entire screen for scrolling"
        (None | Some(0), None) | (Some(0), Some(0)) => {
            processor.ofs_buf.ansi_parser_support.scroll_region_top = None;
            processor.ofs_buf.ansi_parser_support.scroll_region_bottom = None;
            tracing::trace!("CSI r (DECSTBM): Reset scroll region to full screen");
        }
        // Set specific scroll margins.
        // Handles: ESC [ top ; bottom r where top/bottom are valid line numbers
        // This restricts scrolling to only occur within the specified region
        _ => {
            // Set scrolling region with bounds checking.
            let top_row = maybe_top.map_or(
                /* None -> 1 */ 1,
                /* Some(v) -> max(v,1) */ |v| u16::max(v, 1),
            );
            let bottom_row = maybe_bottom.map_or(
                /* None -> buffer_height */ buffer_height,
                /* Some(v) -> min(v,buffer_height) */
                |v| u16::min(v, buffer_height),
            );

            if top_row < bottom_row && bottom_row <= buffer_height {
                processor.ofs_buf.ansi_parser_support.scroll_region_top =
                    Some(term_row(top_row));
                processor.ofs_buf.ansi_parser_support.scroll_region_bottom =
                    Some(term_row(bottom_row));
                tracing::trace!(
                    "CSI r (DECSTBM): Set scroll region from row {} to row {}",
                    top_row,
                    bottom_row
                );
            } else {
                tracing::warn!(
                    "CSI r (DECSTBM): Invalid margins top={}, bottom={}, buffer_height={}",
                    top_row,
                    bottom_row,
                    buffer_height
                );
            }
        }
    }
}
