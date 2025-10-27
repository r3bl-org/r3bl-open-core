// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation] for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_dsr_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_dsr_ops`] - Full pipeline testing via public API
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
//! Application sends "ESC[6n" (request cursor position)
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
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)       ╭───────────╮
//!       - dsr_ops:: for device status (n) <- │THIS MODULE│
//!         ↓                                  ╰───────────╯
//!     Update OffscreenBuffer state
//! ```
//!
//! [`impl_dsr_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_dsr_ops
//! [`test_dsr_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_dsr_ops
//! [module-level documentation]: super::super
//! [operations module documentation]: super
//! [`vt_100_ansi_conformance_tests`]: super::super::vt_100_ansi_conformance_tests

use crate::core::ansi::{generator::DsrRequestType,
                        parser::ansi_parser_public_api::AnsiToOfsBufPerformer};

/// Handle Device Status Report (CSI n) command.
///
/// This command is used by applications to query the terminal's status.
/// Generates DSR response events that will be processed by the process manager
/// and sent back to the child process through the PTY input channel.
pub fn status_report(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    match DsrRequestType::from(params) {
        DsrRequestType::RequestStatus => {
            performer.ofs_buf.handle_status_report_request();
        }
        DsrRequestType::RequestCursorPosition => {
            performer.ofs_buf.handle_cursor_position_request();
        }
        DsrRequestType::Other(n) => {
            tracing::warn!("CSI {}n (DSR): Unsupported device status report request", n);
        }
    }
}
