// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor movement operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the operations module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_cursor_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_cursor_ops`] - Full pipeline testing via public API
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
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │────▶ PTY Master    │────▶ VTE Parser      │────▶ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                            │              ╰──────────────╯
//!        │                                            │                      │
//!        │                                   ╔════════▼════════╗             │
//!        │                                   ║ Perform Trait   ║             │
//!        │                                   ║ Implementation  ║             │
//!        │                                   ╚═════════════════╝             │
//!        │                                                                   │
//!        │                                   ╭─────────────────╮             │
//!        │                                   │ RenderPipeline  ◀─────────────╯
//!        │                                   │ paint()         │
//!        ╰───────────────────────────────────▶ Terminal Output │
//!                                            ╰─────────────────╯
//! ```
//!
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[2A" (cursor up 2 lines)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to operations module:                  ╭───────────╮
//!       - cursor_ops:: for movement (A,B,C,D,H) <- │THIS MODULE│
//!       - scroll_ops:: for scrolling (S,T)         ╰───────────╯
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! # VT100 Protocol Conventions
//!
//! This shim layer sits at the boundary between VT100 wire format and internal types.
//! Understanding VT100 parameter conventions is essential for maintaining this code.
//!
//! ## Parameter Handling
//!
//! **Missing or zero parameters default to 1:**
//! - `ESC[A` (missing param) → interpreted as 1
//! - `ESC[0A` (explicit zero) → interpreted as 1
//! - `ESC[5A` (explicit value) → interpreted as 5
//!
//! This is handled by [`extract_nth_single_non_zero()`] which returns [`NonZeroU16`].
//!
//! ## Coordinate Systems
//!
//! **VT100 uses 1-based coordinates, internal buffers use 0-based indices:**
//!
//! ```text
//! VT100 Wire Format    →    1-based Types    →    0-based Indices
//! ─────────────────         ──────────────         ───────────────
//! ESC[5;10H                 TermRow(5)             RowIndex(4)
//!                           TermCol(10)            ColIndex(9)
//! ```
//!
//! Conversion flow:
//! 1. [`extract_nth_single_non_zero()`] → [`NonZeroU16`] (>= 1)
//! 2. [`TermRow::from_raw_non_zero_value()`] → 1-based coordinate
//! 3. [`.to_zero_based()`] → 0-based buffer index ([`RowIndex`]/[`ColIndex`])
//!
//! [`impl_cursor_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_cursor_ops
//! [`test_cursor_ops`]: crate::core::ansi::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_cursor_ops
//! [module-level documentation]: self
//! [`extract_nth_single_non_zero()`]: crate::ParamsExt::extract_nth_single_non_zero
//! [`NonZeroU16`]: std::num::NonZeroU16
//! [`TermRow::from_raw_non_zero_value()`]: crate::TermRow::from_raw_non_zero_value
//! [`TermCol::from_raw_non_zero_value()`]: crate::TermCol::from_raw_non_zero_value
//! [`.to_zero_based()`]: crate::TermRow::to_zero_based
//! [`RowIndex`]: crate::RowIndex
//! [`ColIndex`]: crate::ColIndex

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::parse_cursor_position};
use crate::{ParamsExt, TermCol, TermRow};
use vte::Params;

/// Move cursor up by n lines.
///
/// **VT100 Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1).
///
/// **Behavior**: Respects DECSTBM scroll region margins.
///
/// **Implementation**: See [`OffscreenBuffer::cursor_up`] for detailed behavior.
///
/// [module-level documentation]: self
/// [`OffscreenBuffer::cursor_up`]: crate::OffscreenBuffer::cursor_up
pub fn cursor_up(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    performer.ofs_buf.cursor_up(how_many);
}

/// Move cursor down by n lines.
///
/// **VT100 Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1).
///
/// **Behavior**: Respects DECSTBM scroll region margins.
///
/// **Implementation**: See [`OffscreenBuffer::cursor_down`] for detailed behavior.
///
/// [module-level documentation]: self
/// [`OffscreenBuffer::cursor_down`]: crate::OffscreenBuffer::cursor_down
pub fn cursor_down(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    performer.ofs_buf.cursor_down(how_many);
}

/// Move cursor forward by n columns.
///
/// **VT100 Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1).
///
/// **Implementation**: See [`OffscreenBuffer::cursor_forward`] for detailed behavior.
///
/// [module-level documentation]: self
/// [`OffscreenBuffer::cursor_forward`]: crate::OffscreenBuffer::cursor_forward
pub fn cursor_forward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    performer.ofs_buf.cursor_forward(how_many);
}

/// Move cursor backward by n columns.
///
/// **VT100 Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1).
///
/// **Implementation**: See [`OffscreenBuffer::cursor_backward`] for detailed behavior.
///
/// [module-level documentation]: self
/// [`OffscreenBuffer::cursor_backward`]: crate::OffscreenBuffer::cursor_backward
pub fn cursor_backward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    performer.ofs_buf.cursor_backward(how_many);
}

/// Set cursor position to (row, column).
///
/// **VT100 Protocol**: See [module-level documentation] for coordinate conversion
/// from 1-based VT100 format to 0-based buffer indices. Missing/zero parameters default
/// to 1.
///
/// **Behavior**: Respects DECSTBM scroll region margins. Coordinates are converted from
/// 1-based VT100 format to 0-based internal indices.
///
/// [module-level documentation]: self
pub fn cursor_position(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let (row, col) = parse_cursor_position(params);
    performer.ofs_buf.cursor_to_position(row, col);
}

/// Handle CNL (Cursor Next Line) - move cursor to beginning of line n lines down.
///
/// **VT100 Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1).
///
/// [module-level documentation]: self
pub fn cursor_next_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    performer.ofs_buf.cursor_down(how_many);
    performer.ofs_buf.cursor_to_line_start();
}

/// Handle CPL (Cursor Previous Line) - move cursor to beginning of line n lines up.
///
/// **VT100 Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1).
///
/// [module-level documentation]: self
pub fn cursor_prev_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    performer.ofs_buf.cursor_up(how_many);
    performer.ofs_buf.cursor_to_line_start();
}

/// Handle CHA (Cursor Horizontal Absolute) - move cursor to column n.
///
/// **VT100 Protocol**: See [module-level documentation] for coordinate conversion
/// from 1-based VT100 format to 0-based buffer indices. Missing/zero parameters default
/// to 1.
///
/// [module-level documentation]: self
pub fn cursor_column(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let target_col =
        TermCol::from_raw_non_zero_value(params.extract_nth_single_non_zero(0))
            .to_zero_based();
    performer.ofs_buf.cursor_to_column(target_col);
}

/// Handle SCP (Save Cursor Position) - save current cursor position.
pub fn save_cursor_position(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.save_cursor_position();
}

/// Handle RCP (Restore Cursor Position) - restore saved cursor position.
pub fn restore_cursor_position(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.restore_cursor_position();
}

/// Handle VPA (Vertical Position Absolute) - move cursor to specified row.
///
/// **VT100 Protocol**: See [module-level documentation] for coordinate conversion
/// from 1-based VT100 format to 0-based buffer indices. Missing/zero parameters default
/// to 1.
///
/// The horizontal position remains unchanged.
///
/// [module-level documentation]: self
pub fn vertical_position_absolute(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
) {
    let target_row =
        TermRow::from_raw_non_zero_value(params.extract_nth_single_non_zero(0))
            .to_zero_based();
    performer.ofs_buf.cursor_to_row(target_row);
}
