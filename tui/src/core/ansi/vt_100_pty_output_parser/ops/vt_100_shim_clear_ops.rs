// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # [`VT-100`] Clear and Erase Operations
//!
//! This module translates [`VT-100`] [`CSI`] escape sequences for erasing text on the
//! display into operations on the [`OffscreenBuffer`].
//!
//! These functions are called by the [`vte`] parser's [`csi_dispatch`] method when
//! specific sequence characters are encountered (e.g., `J` for Erase in Display, `K` for
//! Erase in Line).
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in the conformance tests validating the full pipeline
//!
//! For the complete testing philosophy and rationale behind this approach, see the
//! [ops module].
//!
//! # Architecture Overview
//!
//! See the [module-level Architecture Overview].
//!
//! # [`CSI`] Sequence Processing Flow
//!
//! ```text
//! [Raw Bytes: "ESC [ 2 J"]
//!        ↓
//! [vte Parser]
//!        ↓
//! csi_dispatch() [routes to modules below]
//!        ↓
//! [This Module: erase_in_display(params: [2])]
//!        ↓
//! [OffscreenBuffer: erase_display_entire()]
//! ```
//!
//! ## Erase in Display (`ED` / `CSI n J`)
//!
//! Handled by [`erase_in_display`]. Clears parts of the screen relative to the cursor:
//! - `CSI 0 J` (or just `CSI J`): Clear from cursor to the end of the screen.
//! - `CSI 1 J`: Clear from the beginning of the screen to the cursor.
//! - `CSI 2 J`: Clear the entire screen.
//!
//! ## Erase in Line (`EL` / `CSI n K`)
//!
//! Handled by [`erase_in_line`]. Clears parts of the current line relative to the cursor:
//! - `CSI 0 K` (or just `CSI K`): Clear from cursor to the end of the line.
//! - `CSI 1 K`: Clear from the beginning of the line to the cursor.
//! - `CSI 2 K`: Clear the entire line.
//!
//! [`csi_dispatch`]: crate::AnsiToOfsBufPerformer#method.csi_dispatch
//! [`CSI`]: crate::CsiSequence
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vte`]: https://docs.rs/vte
//! [module-level Architecture Overview]: super#architecture-overview
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::{DEBUG_TUI_VT100_PARSER, ED_ERASE_ALL, ED_ERASE_ALL_AND_SCROLLBACK,
            ED_ERASE_FROM_START, ED_ERASE_TO_END, EL_ERASE_ALL, EL_ERASE_FROM_START,
            EL_ERASE_TO_END, ParamsExt, ok};

/// Handle ED (Erase in Display) - clear screen relative to cursor.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Depending on the first parameter, it clears:
/// - `0` (or default): Cursor to end of screen.
/// - `1`: Beginning of screen to cursor.
/// - `2`: Entire screen.
///
/// See [`OfsBufVT100::erase_display_from_cursor_to_end`],
/// [`OfsBufVT100::erase_display_from_start_to_cursor`], and
/// [`OfsBufVT100::erase_display_entire`] for the implementations of this shim.
///
/// [`OfsBufVT100::erase_display_entire`]:
///     crate::OfsBufVT100::erase_display_entire
/// [`OfsBufVT100::erase_display_from_cursor_to_end`]:
///     crate::OfsBufVT100::erase_display_from_cursor_to_end
/// [`OfsBufVT100::erase_display_from_start_to_cursor`]:
///     crate::OfsBufVT100::erase_display_from_start_to_cursor
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn erase_in_display(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let mode = params.extract_nth_single_opt_raw(0).unwrap_or(0);
    let result = match mode {
        ED_ERASE_TO_END => performer.ofs_buf_vt_100.erase_display_from_cursor_to_end(),
        ED_ERASE_FROM_START => performer
            .ofs_buf_vt_100
            .erase_display_from_start_to_cursor(),
        ED_ERASE_ALL => performer.ofs_buf_vt_100.erase_display_entire(),
        ED_ERASE_ALL_AND_SCROLLBACK => performer
            .ofs_buf_vt_100
            .erase_display_entire_and_scrollback(),
        _ => {
            DEBUG_TUI_VT100_PARSER.then(|| {
                tracing::warn!("CSI {} J: Unsupported Erase Display mode", mode);
            });
            ok!()
        }
    };
    if let Err(err) = result {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::error!("Failed to erase display (mode {}): {:?}", mode, err);
        });
    }
}

/// Handle EL (Erase in Line) - clear line relative to cursor.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Depending on the first parameter, it clears:
/// - `0` (or default): Cursor to end of line.
/// - `1`: Beginning of line to cursor.
/// - `2`: Entire line.
///
/// See [`OfsBufVT100::erase_line_from_cursor_to_end`],
/// [`OfsBufVT100::erase_line_from_start_to_cursor`], and
/// [`OfsBufVT100::erase_line_entire`] for the implementations of this shim.
///
/// [`OfsBufVT100::erase_line_entire`]: crate::OfsBufVT100::erase_line_entire
/// [`OfsBufVT100::erase_line_from_cursor_to_end`]:
///     crate::OfsBufVT100::erase_line_from_cursor_to_end
/// [`OfsBufVT100::erase_line_from_start_to_cursor`]:
///     crate::OfsBufVT100::erase_line_from_start_to_cursor
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn erase_in_line(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let mode = params
        .extract_nth_single_opt_raw(0)
        .unwrap_or(EL_ERASE_TO_END);
    let result = match mode {
        EL_ERASE_TO_END => performer.ofs_buf_vt_100.erase_line_from_cursor_to_end(),
        EL_ERASE_FROM_START => performer.ofs_buf_vt_100.erase_line_from_start_to_cursor(),
        EL_ERASE_ALL => performer.ofs_buf_vt_100.erase_line_entire(),
        _ => {
            DEBUG_TUI_VT100_PARSER.then(|| {
                tracing::warn!("CSI {} K: Unsupported Erase Line mode", mode);
            });
            ok!()
        }
    };
    if let Err(err) = result {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::error!("Failed to erase line (mode {}): {:?}", mode, err);
        });
    }
}
