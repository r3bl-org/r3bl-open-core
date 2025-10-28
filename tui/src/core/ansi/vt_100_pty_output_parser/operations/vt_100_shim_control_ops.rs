// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the operations module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_control_ops`] - Business logic with unit tests
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
//! # Control Character Processing Flow
//!
//! ```text
//! Application sends '\n' (0x0A Line Feed)
//!         ↓
//!     PTY Slave (control character)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (identifies C0 control chars)
//!         ↓
//!     execute() [routes to functions below]
//!         ↓
//!     Route to control operations:                          ╭───────────╮
//!       - control_ops:: for control chars (BS,TAB,LF,CR) <- │THIS MODULE│
//!         ↓                                                 ╰───────────╯
//!     Update OffscreenBuffer state
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
//! parameters, unlike their CSI sequence counterparts.
//!
//! [`impl_control_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_control_ops
//! [`test_control_ops`]: crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::tests::vt_100_test_control_ops
//! [module-level documentation]: self

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;

/// Handle BS (Backspace) - move cursor left one position.
/// If cursor is at start of line, behavior depends on terminal settings.
/// See [`OffscreenBuffer::handle_backspace`] for detailed behavior.
///
/// [`OffscreenBuffer::handle_backspace`]: crate::OffscreenBuffer::handle_backspace
pub fn handle_backspace(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_backspace();
}

/// Handle TAB (Horizontal Tab) - move cursor to next tab stop.
/// Tab stops are typically at columns 0, 8, 16, 24, 32, etc.
/// See [`OffscreenBuffer::handle_tab`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::handle_tab`]: crate::OffscreenBuffer::handle_tab
pub fn handle_tab(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_tab();
}

/// Handle LF (Line Feed) - move cursor down one line.
/// Cursor column position remains unchanged.
/// See [`OffscreenBuffer::handle_line_feed`] for detailed behavior.
///
/// [`OffscreenBuffer::handle_line_feed`]: crate::OffscreenBuffer::handle_line_feed
pub fn handle_line_feed(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_line_feed();
}

/// Handle CR (Carriage Return) - move cursor to start of current line.
/// Cursor row position remains unchanged.
/// See [`OffscreenBuffer::handle_carriage_return`] for detailed behavior.
///
/// [`OffscreenBuffer::handle_carriage_return`]: crate::OffscreenBuffer::handle_carriage_return
pub fn handle_carriage_return(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_carriage_return();
}
