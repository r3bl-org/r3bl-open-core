// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor movement operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation] for details on the "shim → impl →
//! test" architecture and naming conventions.
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
//! - **Integration tests** in [`vt_100_ansi_conformance_tests`] validating the full
//!   pipeline
//!
//! See the [operations module documentation] for the complete testing philosophy
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
//! [`impl_cursor_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_cursor_ops
//! [`test_cursor_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_cursor_ops
//! [module-level documentation]: super::super
//! [operations module documentation]: super
//! [`vt_100_ansi_conformance_tests`]: super::super::vt_100_ansi_conformance_tests

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::{AbsolutePosition, CursorPositionRequest,
                                          MovementCount}};
use vte::Params;

/// Move cursor up by n lines.
/// Respects DECSTBM scroll region margins.
/// See [`OffscreenBuffer::cursor_up`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::cursor_up`]: crate::OffscreenBuffer::cursor_up
pub fn cursor_up(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_first_as_row_height_non_zero(params);
    performer.ofs_buf.cursor_up(how_many);
}

/// Move cursor down by n lines.
/// Respects DECSTBM scroll region margins.
/// See [`OffscreenBuffer::cursor_down`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::cursor_down`]: crate::OffscreenBuffer::cursor_down
pub fn cursor_down(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_first_as_row_height_non_zero(params);
    performer.ofs_buf.cursor_down(how_many);
}

/// Move cursor forward by n columns.
/// See [`OffscreenBuffer::cursor_forward`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::cursor_forward`]: crate::OffscreenBuffer::cursor_forward
pub fn cursor_forward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_first_as_col_width_non_zero(params);
    performer.ofs_buf.cursor_forward(how_many);
}

/// Move cursor backward by n columns.
/// See [`OffscreenBuffer::cursor_backward`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::cursor_backward`]: crate::OffscreenBuffer::cursor_backward
pub fn cursor_backward(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_first_as_col_width_non_zero(params);
    performer.ofs_buf.cursor_backward(how_many);
}

/// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
/// Respects DECSTBM scroll region margins.
pub fn cursor_position(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let request = CursorPositionRequest::from(params);
    performer.ofs_buf.cursor_to_position(request.row, request.col);
}

/// Handle CNL (Cursor Next Line) - move cursor to beginning of line n lines down.
pub fn cursor_next_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_first_as_row_height_non_zero(params);
    performer.ofs_buf.cursor_down(how_many);
    performer.ofs_buf.cursor_to_line_start();
}

/// Handle CPL (Cursor Previous Line) - move cursor to beginning of line n lines up.
pub fn cursor_prev_line(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let how_many = MovementCount::parse_first_as_row_height_non_zero(params);
    performer.ofs_buf.cursor_up(how_many);
    performer.ofs_buf.cursor_to_line_start();
}

/// Handle CHA (Cursor Horizontal Absolute) - move cursor to column n (1-based).
pub fn cursor_column(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let target_col =
        AbsolutePosition::parse_first_as_col_index_non_zero_to_index_type(params);
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
/// The horizontal position remains unchanged.
/// Row parameter is 1-based, with default value of 1.
pub fn vertical_position_absolute(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
) {
    let target_row =
        AbsolutePosition::parse_first_as_row_index_non_zero_to_index_type(params);
    performer.ofs_buf.cursor_to_row(target_row);
}
