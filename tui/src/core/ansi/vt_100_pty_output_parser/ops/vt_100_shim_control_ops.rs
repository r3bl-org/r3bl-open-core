// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_control_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_control_ops`] - Full pipeline testing via public API
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
//! For the complete testing philosophy and rationale behind this approach,
//! see the [ops module].
//!
//! # Architecture Overview
//!
//! See the [module-level Architecture Overview].
//!
//! # Control Character Processing Flow
//!
//! ```text
//! Application sends '\n' (0x0A Line Feed)
//!         ↓
//!     PTY Controlled (control character)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (identifies C0 control chars)
//!         ↓
//!     execute() [routes to functions below]
//!         ↓
//!     Route to control operations:                             ╭───────────╮
//!       - control_ops:: for control chars (BS,TAB,LF,CR)    <- │THIS MODULE│
//!         ↓                                                    ╰───────────╯
//!     Update OfsBuf state
//! ```
//!
//! # Supported Control Characters
//!
//! This module handles the basic C0 control characters that are essential for
//! terminal operation:
//!
//! - **BS (0x08)**: Backspace - move cursor left one position
//! - **TAB (0x09)**: Horizontal Tab - move cursor to next tab stop
//! - **LF (0x0A)**: Line Feed - move cursor down one line
//! - **CR (0x0D)**: Carriage Return - move cursor to start of current line
//!
//! These operations are fundamental cursor control operations that don't require
//! parameters, unlike their [`CSI`] sequence counterparts.
//!
//! [`CSI`]: crate::CsiSequence
//! [`test_control_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_control_ops
//! [`vt_100_impl_control_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_control_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;

/// Handle BS (Backspace) - move cursor left one position.
/// If cursor is at start of line, behavior depends on terminal settings.
/// See [`OfsBufVT100::handle_backspace`] for detailed behavior.
///
/// [`OfsBufVT100::handle_backspace`]: crate::OfsBufVT100::handle_backspace
pub fn handle_backspace(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.handle_backspace();
}

/// Handle TAB (Horizontal Tab) - move cursor to next tab stop.
/// Tab stops are typically at columns 0, 8, 16, 24, 32, etc.
/// See [`OfsBufVT100::handle_tab`] for detailed behavior and examples.
///
/// [`OfsBufVT100::handle_tab`]: crate::OfsBufVT100::handle_tab
pub fn handle_tab(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.handle_tab();
}

/// Handle LF (Line Feed) - move cursor down one line.
/// Cursor column position remains unchanged.
/// See [`OfsBufVT100::handle_line_feed`] for detailed behavior.
///
/// [`OfsBufVT100::handle_line_feed`]: crate::OfsBufVT100::handle_line_feed
pub fn handle_line_feed(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.handle_line_feed();
}

/// Handle CR (Carriage Return) - move cursor to start of current line.
/// Cursor row position remains unchanged.
/// See [`OfsBufVT100::handle_carriage_return`] for detailed behavior.
///
/// [`OfsBufVT100::handle_carriage_return`]: crate::OfsBufVT100::handle_carriage_return
pub fn handle_carriage_return(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.handle_carriage_return();
}
