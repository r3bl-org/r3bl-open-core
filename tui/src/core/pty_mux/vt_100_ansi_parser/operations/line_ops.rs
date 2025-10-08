// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line insertion and deletion operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation] for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_line_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_line_ops`] - Full pipeline testing via public API
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
//! Application sends "ESC[2L" (insert 2 lines)
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
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)     ╭───────────╮
//!       - line_ops:: for lines (L,M) <- │THIS MODULE│
//!       - char_ops:: for chars (@,P,X)  ╰───────────╯
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! [`impl_line_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_line_ops
//! [`test_line_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::test_line_ops
//! [module-level documentation]: super::super
//! [operations module documentation]: super
//! [`vt_100_ansi_conformance_tests`]: super::super::vt_100_ansi_conformance_tests

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount};

/// Handle IL (Insert Line) - insert n blank lines at cursor position.
/// Lines below cursor and within scroll region shift down.
///
/// This operation respects VT-100 scroll region boundaries.
/// See [`OffscreenBuffer::insert_lines_at`] for detailed behavior and scroll region
/// handling.
///
/// [`OffscreenBuffer::insert_lines_at`]: crate::OffscreenBuffer::insert_lines_at
pub fn insert_lines(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_first_as_row_height_non_zero(params);
    let at = performer.ofs_buf.cursor_pos.row_index;
    let result = performer.ofs_buf.insert_lines_at(at, how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to insert {how_many:?} lines at row {at:?}",
    );
}

/// Handle DL (Delete Line) - delete n lines starting at cursor position.
/// Lines below cursor and within scroll region shift up.
/// Blank lines are added at the bottom of the scroll region.
///
/// This operation respects VT-100 scroll region boundaries.
/// See [`OffscreenBuffer::delete_lines_at`] for detailed behavior and scroll region
/// handling.
///
/// [`OffscreenBuffer::delete_lines_at`]: crate::OffscreenBuffer::delete_lines_at
pub fn delete_lines(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_first_as_row_height_non_zero(params);
    let at = performer.ofs_buf.cursor_pos.row_index;
    let result = performer.ofs_buf.delete_lines_at(at, how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to delete {how_many:?} lines at row {at:?}",
    );
}
